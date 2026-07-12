
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::voucher_service::{CreateVoucherRequest, VoucherItemRequest, VoucherService};
use crate::utils::error::AppError;
use futures::FutureExt;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tracing::{error, info};

/// 库存财务桥接服务
/// 负责监听库存变动事件并自动生成相应的会计凭证
pub struct InventoryFinanceBridgeService {
    db: Arc<DatabaseConnection>,
}

/// 凭证分录构造参数对象
///
/// 批次 334 v10 复审 P3 修复：引入参数对象消除 make_voucher_item 的 too_many_arguments 警告。
/// 聚合凭证分录所需的全部字段，避免函数签名携带 9 个参数。
/// 使用生命周期 `'_` 借用 subject_code / subject_name，避免调用方不必要的 to_string()。
pub struct VoucherItemArgs<'a> {
    /// 行号
    pub line_no: i32,
    /// 科目编码
    pub subject_code: &'a str,
    /// 科目名称
    pub subject_name: &'a str,
    /// 借方金额
    pub debit: Decimal,
    /// 贷方金额
    pub credit: Decimal,
    /// 摘要
    pub summary: Option<String>,
    /// 数量（米）
    pub quantity_meters: Option<Decimal>,
    /// 数量（公斤）
    pub quantity_kg: Option<Decimal>,
    /// 单价
    pub unit_price: Option<Decimal>,
}

/// 库存事件生成凭证参数对象
///
/// 批次 337 v10 复审 P3 修复：引入参数对象消除 5 个 create_*_voucher 私有函数的 too_many_arguments 警告。
/// 5 个函数（create_purchase_receipt_voucher / create_sales_delivery_voucher /
/// create_inventory_adjustment_voucher / create_production_receipt_voucher /
/// create_production_issue_voucher）参数完全一致，统一聚合为单一参数对象。
/// 使用生命周期 `'_` 借用 source_bill_type / source_bill_no / batch_no / color_no，
/// 避免调用方不必要的 to_string()。
pub struct VoucherCreateArgs<'a> {
    /// 产品 ID
    pub product_id: i32,
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
    /// 来源单据类型（可选）
    pub source_bill_type: Option<&'a str>,
    /// 来源单据号（可选）
    pub source_bill_no: Option<&'a str>,
    /// 来源单据 ID（可选）
    pub source_bill_id: Option<i32>,
    /// 批次号
    pub batch_no: &'a str,
    /// 色号
    pub color_no: &'a str,
    /// 创建人 ID（可选，系统自动生成时为 None）
    pub created_by: Option<i32>,
}

impl InventoryFinanceBridgeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建默认凭证分录（填充所有必填字段）
    ///
    /// 批次 334 v10 复审 P3 修复：签名从 9 参数改为单一参数对象 `VoucherItemArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    fn make_voucher_item(args: VoucherItemArgs<'_>) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(args.line_no),
            subject_code: Some(args.subject_code.to_string()),
            subject_name: Some(args.subject_name.to_string()),
            debit: args.debit,
            credit: args.credit,
            summary: args.summary,
            assist_customer_id: None,
            assist_supplier_id: None,
            assist_department_id: None,
            assist_employee_id: None,
            assist_project_id: None,
            assist_batch_id: None,
            assist_color_no_id: None,
            assist_dye_lot_id: None,
            assist_grade: None,
            assist_workshop_id: None,
            quantity_meters: args.quantity_meters,
            quantity_kg: args.quantity_kg,
            unit_price: args.unit_price,
        }
    }

    /// 启动库存变动事件监听器
    pub fn start_listener(db: Arc<DatabaseConnection>) {
        let mut receiver = EVENT_BUS.subscribe();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                // 批次 8（2026-06-28）：单次事件处理 panic 隔离
                // 库存财务桥接监听器 panic 会导致库存交易不再生成会计凭证，
                // 财务报表与库存数据不一致。
                let result = AssertUnwindSafe(async {
                if let BusinessEvent::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                } = event
                {
                    info!(
                        "处理库存交易创建事件: 交易ID={}, 类型={}, 产品ID={}, 仓库ID={}",
                        transaction_id, transaction_type, product_id, warehouse_id
                    );

                    let bridge_service = InventoryFinanceBridgeService::new(db.clone());
                    // 批次 337 v10 复审 P3 修复：使用 VoucherCreateArgs 参数对象替代多参数
                    let voucher_args = VoucherCreateArgs {
                        product_id,
                        warehouse_id,
                        quantity_meters,
                        quantity_kg,
                        source_bill_type: source_bill_type.as_deref(),
                        source_bill_no: source_bill_no.as_deref(),
                        source_bill_id,
                        batch_no: &batch_no,
                        color_no: &color_no,
                        created_by,
                    };
                    if let Err(e) = bridge_service
                        .handle_inventory_transaction(
                            transaction_id,
                            &transaction_type,
                            voucher_args,
                        )
                        .await
                    {
                        error!(
                            "处理库存交易事件失败: 交易ID={}, 错误={}",
                            transaction_id, e
                        );
                    }
                }
                })
                .catch_unwind()
                .await;
                if let Err(panic_payload) = result {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ 库存财务桥接 spawn panic 已被隔离，继续运行（不退出循环）"
                    );
                }
            }
        });
    }

    /// 处理库存交易事件，生成相应的会计凭证
    ///
    /// 批次 337 v10 复审 P3 修复：签名从 12 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。transaction_type 改为 args 内嵌字段处理
    /// 不再单独传递，通过 match 分发到 5 个 create_*_voucher 私有函数。
    async fn handle_inventory_transaction(
        &self,
        _transaction_id: i32,
        transaction_type: &str,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        // 根据交易类型生成不同的凭证
        match transaction_type {
            "PURCHASE_RECEIPT" => {
                // 采购入库凭证：借：库存商品 / 贷：应付账款
                self.create_purchase_receipt_voucher(args).await?;
            }
            "SALES_DELIVERY" => {
                // 销售出库凭证：借：主营业务成本 / 贷：库存商品
                self.create_sales_delivery_voucher(args).await?;
            }
            "INVENTORY_ADJUSTMENT" => {
                // 库存调整凭证
                self.create_inventory_adjustment_voucher(args).await?;
            }
            "PRODUCTION_RECEIPT" => {
                // 生产入库凭证：借：库存商品 / 贷：生产成本
                self.create_production_receipt_voucher(args).await?;
            }
            "PRODUCTION_ISSUE" => {
                // 生产领料凭证：借：生产成本 / 贷：库存商品
                self.create_production_issue_voucher(args).await?;
            }
            _ => {
                info!("未处理的库存交易类型: {}", transaction_type);
            }
        }

        Ok(())
    }

    /// 创建采购入库凭证
    /// 借：库存商品
    /// 贷：应付账款
    /// 批次 337 v10 复审 P3 修复：签名从 10 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    async fn create_purchase_receipt_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        let VoucherCreateArgs {
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = args;
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证，
        // 避免 amount / quantity_meters 裸除法触发 panic 导致监听器任务异常
        if quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        // P2 5-12 修复：合并产品名称+成本价为单次查询（原为 2 次 product 查询）
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));

        let summary = format!(
            "采购入库：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        );

        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * quantity_meters).round_dp(2);

        let voucher_request = CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id,
            source_bill_no: source_bill_no.map(|s| s.to_string()),
            batch_no: Some(batch_no.to_string()),
            color_no: Some(color_no.to_string()),
            items: vec![
                // 借：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(quantity_meters),
                    quantity_kg: Some(quantity_kg),
                    // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                    unit_price: Some((amount / quantity_meters).round_dp(2)),
                }),
                // 贷：应付账款
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "2202",
                    subject_name: "应付账款",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        // created_by 缺失时拒绝生成凭证，避免财务记录归到 user_id=0 系统用户
        let user_id =
            created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成采购入库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 创建销售出库凭证
    /// 借：主营业务成本
    /// 贷：库存商品
    /// 批次 337 v10 复审 P3 修复：签名从 10 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    async fn create_sales_delivery_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        let VoucherCreateArgs {
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = args;
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证，
        // 避免 amount / quantity_meters 裸除法触发 panic 导致监听器任务异常
        if quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        // P2 5-12 修复：合并产品名称+成本价为单次查询
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));

        let summary = format!(
            "销售出库：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        );

        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * quantity_meters).round_dp(2);

        let voucher_request = CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id,
            source_bill_no: source_bill_no.map(|s| s.to_string()),
            batch_no: Some(batch_no.to_string()),
            color_no: Some(color_no.to_string()),
            items: vec![
                // 借：主营业务成本
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "6401",
                    subject_name: "主营业务成本",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(quantity_meters),
                    quantity_kg: Some(quantity_kg),
                    // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                    unit_price: Some((amount / quantity_meters).round_dp(2)),
                }),
                // 贷：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(quantity_meters),
                    quantity_kg: Some(quantity_kg),
                    // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                    unit_price: Some((amount / quantity_meters).round_dp(2)),
                }),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        // created_by 缺失时拒绝生成凭证，避免财务记录归到 user_id=0 系统用户
        let user_id =
            created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成销售出库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 创建库存调整凭证
    /// 盘盈：借：库存商品 / 贷：待处理财产损溢
    /// 盘亏：借：待处理财产损溢 / 贷：库存商品
    /// 批次 337 v10 复审 P3 修复：签名从 10 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    async fn create_inventory_adjustment_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        let VoucherCreateArgs {
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = args;
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证，
        // 避免 amount / quantity_meters 裸除法触发 panic 导致监听器任务异常
        if quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        // P2 5-12 修复：合并产品名称+成本价为单次查询
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));

        let summary = format!(
            "库存调整：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        );

        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * quantity_meters.abs()).round_dp(2);

        let voucher_request = if quantity_meters > Decimal::ZERO {
            // 盘盈：借：库存商品，贷：待处理财产损溢
            CreateVoucherRequest {
                voucher_type: "记".to_string(),
                voucher_date: chrono::Utc::now().date_naive(),
                source_type: source_bill_type.map(|s| s.to_string()),
                source_module: Some("inventory".to_string()),
                source_bill_id,
                source_bill_no: source_bill_no.map(|s| s.to_string()),
                batch_no: Some(batch_no.to_string()),
                color_no: Some(color_no.to_string()),
                items: vec![
                    Self::make_voucher_item(VoucherItemArgs {
                        line_no: 1,
                        subject_code: "1405",
                        subject_name: "库存商品",
                        debit: amount,
                        credit: Decimal::ZERO,
                        summary: Some(summary.clone()),
                        quantity_meters: Some(quantity_meters),
                        quantity_kg: Some(quantity_kg),
                        // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                        unit_price: Some((amount / quantity_meters).round_dp(2)),
                    }),
                    Self::make_voucher_item(VoucherItemArgs {
                        line_no: 2,
                        subject_code: "1901",
                        subject_name: "待处理财产损溢",
                        debit: Decimal::ZERO,
                        credit: amount,
                        summary: Some(summary.clone()),
                        quantity_meters: None,
                        quantity_kg: None,
                        unit_price: None,
                    }),
                ],
            }
        } else {
            // 盘亏：借：待处理财产损溢，贷：库存商品
            CreateVoucherRequest {
                voucher_type: "记".to_string(),
                voucher_date: chrono::Utc::now().date_naive(),
                source_type: source_bill_type.map(|s| s.to_string()),
                source_module: Some("inventory".to_string()),
                source_bill_id,
                source_bill_no: source_bill_no.map(|s| s.to_string()),
                batch_no: Some(batch_no.to_string()),
                color_no: Some(color_no.to_string()),
                items: vec![
                    Self::make_voucher_item(VoucherItemArgs {
                        line_no: 1,
                        subject_code: "1901",
                        subject_name: "待处理财产损溢",
                        debit: amount,
                        credit: Decimal::ZERO,
                        summary: Some(summary.clone()),
                        quantity_meters: None,
                        quantity_kg: None,
                        unit_price: None,
                    }),
                    Self::make_voucher_item(VoucherItemArgs {
                        line_no: 2,
                        subject_code: "1405",
                        subject_name: "库存商品",
                        debit: Decimal::ZERO,
                        credit: amount,
                        summary: Some(summary.clone()),
                        quantity_meters: Some(-quantity_meters),
                        quantity_kg: Some(-quantity_kg),
                        // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                        unit_price: Some((amount / (-quantity_meters)).round_dp(2)),
                    }),
                ],
            }
        };

        let voucher_service = VoucherService::new(self.db.clone());
        // created_by 缺失时拒绝生成凭证，避免财务记录归到 user_id=0 系统用户
        let user_id =
            created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成库存调整凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 创建生产入库凭证
    /// 借：库存商品 / 贷：生产成本
    /// 批次 337 v10 复审 P3 修复：签名从 10 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    async fn create_production_receipt_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        let VoucherCreateArgs {
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = args;
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证，
        // 避免 amount / quantity_meters 裸除法触发 panic 导致监听器任务异常
        if quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        // P2 5-12 修复：合并产品名称+成本价为单次查询
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));

        let summary = format!(
            "生产入库：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        );

        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * quantity_meters).round_dp(2);

        let voucher_request = CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id,
            source_bill_no: source_bill_no.map(|s| s.to_string()),
            batch_no: Some(batch_no.to_string()),
            color_no: Some(color_no.to_string()),
            items: vec![
                // 借：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(quantity_meters),
                    quantity_kg: Some(quantity_kg),
                    // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                    unit_price: Some((amount / quantity_meters).round_dp(2)),
                }),
                // 贷：生产成本
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "5001",
                    subject_name: "生产成本",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        // created_by 缺失时拒绝生成凭证，避免财务记录归到 user_id=0 系统用户
        let user_id =
            created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成生产入库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 创建生产领料凭证
    /// 借：生产成本 / 贷：库存商品
    /// 批次 337 v10 复审 P3 修复：签名从 10 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
    async fn create_production_issue_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        let VoucherCreateArgs {
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = args;
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证，
        // 避免 amount / quantity_meters 裸除法触发 panic 导致监听器任务异常
        if quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        // P2 5-12 修复：合并产品名称+成本价为单次查询
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));

        let summary = format!(
            "生产领料：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        );

        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * quantity_meters).round_dp(2);

        let voucher_request = CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id,
            source_bill_no: source_bill_no.map(|s| s.to_string()),
            batch_no: Some(batch_no.to_string()),
            color_no: Some(color_no.to_string()),
            items: vec![
                // 借：生产成本
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "5001",
                    subject_name: "生产成本",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(quantity_meters),
                    quantity_kg: Some(quantity_kg),
                    // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                    unit_price: Some((amount / quantity_meters).round_dp(2)),
                }),
                // 贷：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(quantity_meters),
                    quantity_kg: Some(quantity_kg),
                    // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
                    unit_price: Some((amount / quantity_meters).round_dp(2)),
                }),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        // created_by 缺失时拒绝生成凭证，避免财务记录归到 user_id=0 系统用户
        let user_id =
            created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成生产领料凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 获取仓库名称
    async fn get_warehouse_name(&self, warehouse_id: i32) -> Result<String, AppError> {
        use crate::models::warehouse;
        use sea_orm::EntityTrait;

        let warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("仓库不存在: {}", warehouse_id)))?;

        Ok(warehouse.name)
    }

    /// 一次性获取产品名称与成本价（P2 5-12 修复：合并原 get_product_name + get_product_cost_price 两次查询为单次）
    /// 返回 (name, cost_price)；产品不存在时返回 Err（与原 calculate_inventory_amount 行为一致）
    async fn get_product_info(&self, product_id: i32) -> Result<(String, Decimal), AppError> {
        use crate::models::product;
        use sea_orm::EntityTrait;

        let product = product::Entity::find_by_id(product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品不存在: {}", product_id)))?;

        // P2 3-17 修复：原 cost_price.unwrap_or(Decimal::ZERO) 在产品未设置成本价时
        // 静默返回 0，导致 calculate_inventory_amount 金额计算为 0 却无任何提示，
        // 财务报表失真。改为记录 warn 日志，仍返回 ZERO 不阻断流程（避免破坏现有批次），
        // 但留下审计痕迹便于运维排查。
        let cost_price = product.cost_price.unwrap_or(Decimal::ZERO);
        if cost_price <= Decimal::ZERO {
            tracing::warn!(
                product_id,
                product_name = %product.name,
                "P2 3-17: 产品未设置成本价，金额计算将为 0，请先维护成本价"
            );
        }
        Ok((product.name, cost_price))
    }
}
