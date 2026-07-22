
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::voucher_service::{CreateVoucherRequest, VoucherItemRequest, VoucherService};
use crate::utils::error::AppError;
use futures::FutureExt;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tracing::{error, info};

/// L-29 修复（批次 373 v13 复审）：库存财务桥接监听器 spawn 句柄
/// 保存句柄以便 shutdown 时 abort，避免 detached task 泄漏
static BRIDGE_LISTENER_HANDLE: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(None);

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

/// 库存盘盈盘亏凭证构造参数对象
///
/// D08 第三梯队修复：引入参数对象消除 build_overage_voucher_request /
/// build_shortage_voucher_request 两个函数的 too_many_arguments 警告。
/// 两函数签名一致，统一聚合为单一参数对象，避免函数签名携带 9 个参数。
pub struct BridgeVoucherArgs<'a> {
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
    /// 摘要
    pub summary: &'a str,
    /// 金额
    pub amount: Decimal,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
}

/// 采购退货凭证构造上下文
///
/// 拆分 create_purchase_return_voucher 时引入，避免元组返回。
struct PurchaseReturnVoucherContext {
    product_name: String,
    warehouse_name: String,
    cost_price: Decimal,
}

/// 生产领料凭证构造上下文
///
/// 拆分 create_production_issue_voucher 时引入，避免元组返回。
struct ProductionIssueVoucherContext {
    product_name: String,
    warehouse_name: String,
    cost_price: Decimal,
}

/// 销售退货凭证构造上下文
///
/// 拆分 create_sales_return_voucher 时引入，避免元组返回。
struct SalesReturnVoucherContext {
    product_name: String,
    warehouse_name: String,
    cost_price: Decimal,
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

        // L-29 修复（批次 373 v13 复审）：保存 spawn 句柄供 shutdown abort
        let listener_handle = tokio::spawn(async move {
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

        // L-29 修复（批次 373 v13 复审）：保存句柄到全局 static
        if let Ok(mut guard) = BRIDGE_LISTENER_HANDLE.lock() {
            *guard = Some(listener_handle);
        }
    }

    /// L-29 修复（批次 373 v13 复审）：优雅关闭库存财务桥接监听器
    /// abort 后台 spawn task，防止 detached task 泄漏。幂等：多次调用安全。
    pub fn shutdown_listener() {
        let handle = match BRIDGE_LISTENER_HANDLE.lock() {
            Ok(mut guard) => guard.take(),
            Err(e) => {
                tracing::error!(error = %e, "BRIDGE_LISTENER_HANDLE 锁中毒，无法关闭监听器");
                return;
            }
        };
        if let Some(h) = handle {
            h.abort();
            tracing::info!("库存财务桥接监听器 task 已关闭");
        }
    }

    /// 处理库存交易事件，生成相应的会计凭证
    ///
    /// 批次 337 v10 复审 P3 修复：签名从 12 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。transaction_type 改为 args 内嵌字段处理
    /// 不再单独传递，通过 match 分发到 5 个 create_*_voucher 私有函数。
    async fn handle_inventory_transaction(
        &self,
        transaction_id: i32,
        transaction_type: &str,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        // B-P1-8 修复（批次 365 v13 复审）：事件幂等处理
        // 重复消费 InventoryTransactionCreated 会导致重复生成会计凭证 + 重复过账，
        // 科目余额累加失真。使用 transaction_id 作为幂等键，处理前检查是否已处理。
        let idempotency_service =
            crate::services::event_idempotency_service::EventIdempotencyService::new(self.db.clone());
        let consumer_id = "inventory_finance_bridge";
        let event_key = format!("inventory_txn:{}", transaction_id);
        let event_type = "InventoryTransactionCreated";
        let should_process = idempotency_service
            .try_mark_processed(consumer_id, &event_key, event_type)
            .await?;
        if !should_process {
            info!(
                transaction_id = transaction_id,
                transaction_type = transaction_type,
                "库存交易事件已处理，幂等跳过凭证生成"
            );
            return Ok(());
        }

        // 根据交易类型生成不同的凭证
        match transaction_type {
            "PURCHASE_RECEIPT" => {
                // 采购入库凭证：借：库存商品 / 贷：应付账款
                self.create_purchase_receipt_voucher(args).await?;
            }
            "PURCHASE_RETURN" => {
                // 批次 356 v13 复审 B-P0-5 修复：采购退货凭证
                // 借：应付账款（红字） / 贷：库存商品（红字）
                self.create_purchase_return_voucher(args).await?;
            }
            "SALES_DELIVERY" => {
                // 销售出库凭证：借：主营业务成本 / 贷：库存商品
                self.create_sales_delivery_voucher(args).await?;
            }
            "SALES_RETURN" => {
                // 批次 356 v13 复审 B-P0-6 修复：销售退货凭证
                // 借：库存商品 / 贷：主营业务成本（红字反转）
                self.create_sales_return_voucher(args).await?;
            }
            "INVENTORY_ADJUSTMENT" => {
                // 库存调整凭证
                self.create_inventory_adjustment_voucher(args).await?;
            }
            "PRODUCTION_RECEIPT" | "PRODUCTION_OUTPUT" => {
                // 生产入库凭证：借：库存商品 / 贷：生产成本
                // 批次 356 v13 复审 B-P0-4 修复：兼容 PRODUCTION_OUTPUT 事件类型
                self.create_production_receipt_voucher(args).await?;
            }
            "PRODUCTION_ISSUE" | "PRODUCTION_CONSUMPTION" => {
                // 生产领料凭证：借：生产成本 / 贷：库存商品
                // 批次 356 v13 复审 B-P0-4 修复：兼容 PRODUCTION_CONSUMPTION 事件类型
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
        // P0 5-4 修复：除零保护；created_by 缺失拒绝生成凭证
        self.validate_quantity_meters(quantity_meters)?;
        let user_id = self.validate_created_by(created_by)?;
        // P2 5-12 修复：合并产品名称+成本价为单次查询（原为 2 次 product 查询）
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));
        let summary = self.build_purchase_receipt_summary(
            &product_name, quantity_meters, quantity_kg, batch_no, color_no, &warehouse_name,
        );
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * quantity_meters).round_dp(2);
        let voucher_request = self.build_purchase_receipt_voucher_request(BridgeVoucherArgs {
            source_bill_type, source_bill_no, source_bill_id,
            batch_no, color_no, summary: &summary, amount,
            quantity_meters, quantity_kg,
        });
        let voucher_service = VoucherService::new(self.db.clone());
        // 批次 356 v13 复审 F-P0-2 修复：create → create_and_post 自动过账，触发科目余额回写
        let voucher = voucher_service.create_and_post(voucher_request, user_id).await?;
        info!(
            "自动生成采购入库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );
        Ok(())
    }

    /// 构建采购入库凭证摘要
    fn build_purchase_receipt_summary(
        &self,
        product_name: &str,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        batch_no: &str,
        color_no: &str,
        warehouse_name: &str,
    ) -> String {
        format!(
            "采购入库：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        )
    }

    /// 构建采购入库凭证请求体（借：库存商品 / 贷：应付账款），复用 BridgeVoucherArgs 避免新增参数对象
    fn build_purchase_receipt_voucher_request(
        &self,
        args: BridgeVoucherArgs<'_>,
    ) -> CreateVoucherRequest {
        // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
        let unit_price = (args.amount / args.quantity_meters).round_dp(2);
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                // 借：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: args.amount,
                    credit: Decimal::ZERO,
                    summary: Some(args.summary.to_string()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some(unit_price),
                }),
                // 贷：应付账款
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "2202",
                    subject_name: "应付账款",
                    debit: Decimal::ZERO,
                    credit: args.amount,
                    summary: Some(args.summary.to_string()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
            ],
        }
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
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证
        if args.quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        // P2 5-12 修复：合并产品名称+成本价为单次查询
        let (product_name, cost_price) = self
            .get_product_info(args.product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", args.product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(args.warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", args.warehouse_id));
        let summary = format!(
            "销售出库：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, args.quantity_meters, args.quantity_kg, args.batch_no, args.color_no, warehouse_name
        );
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let amount = (cost_price * args.quantity_meters).round_dp(2);
        let voucher_request = Self::build_delivery_voucher_request(&args, summary, amount);
        let voucher_service = VoucherService::new(self.db.clone());
        // created_by 缺失时拒绝生成凭证，避免财务记录归到 user_id=0 系统用户
        let user_id = args.created_by
            .ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        // 批次 356 v13 复审 F-P0-2 修复：create → create_and_post 自动过账，触发科目余额回写
        let voucher = voucher_service.create_and_post(voucher_request, user_id).await?;
        info!(
            "自动生成销售出库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, args.batch_no, args.color_no
        );
        Ok(())
    }

    /// 构建销售出库凭证请求体（借：主营业务成本 / 贷：库存商品），复用 VoucherCreateArgs 避免新增参数对象
    fn build_delivery_voucher_request(
        args: &VoucherCreateArgs<'_>,
        summary: String,
        amount: Decimal,
    ) -> CreateVoucherRequest {
        // P3 维度 4 修复（批次 87）：单价计算补 round_dp(2)
        let unit_price = (amount / args.quantity_meters).round_dp(2);
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                // 借：主营业务成本
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "6401",
                    subject_name: "主营业务成本",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some(unit_price),
                }),
                // 贷：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some(unit_price),
                }),
            ],
        }
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

        self.validate_quantity_meters(quantity_meters)?;
        let user_id = self.validate_created_by(created_by)?;

        let (product_name, cost_price) = self.get_product_info(product_id).await.unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self.get_warehouse_name(warehouse_id).await.unwrap_or_else(|_| format!("仓库{}", warehouse_id));

        let summary = self.build_adjustment_summary(product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name);
        let amount = (cost_price * quantity_meters.abs()).round_dp(2);

        let voucher_request = if quantity_meters > Decimal::ZERO {
            let args = BridgeVoucherArgs {
                source_bill_type,
                source_bill_no,
                source_bill_id,
                batch_no,
                color_no,
                summary: &summary,
                amount,
                quantity_meters,
                quantity_kg,
            };
            self.build_overage_voucher_request(&args)
        } else {
            let args = BridgeVoucherArgs {
                source_bill_type,
                source_bill_no,
                source_bill_id,
                batch_no,
                color_no,
                summary: &summary,
                amount,
                quantity_meters,
                quantity_kg,
            };
            self.build_shortage_voucher_request(&args)
        };

        self.create_and_log_voucher(voucher_request, user_id, batch_no, color_no).await?;

        Ok(())
    }

    fn validate_quantity_meters(&self, quantity_meters: Decimal) -> Result<(), AppError> {
        if quantity_meters.is_zero() {
            return Err(AppError::validation("quantity_meters 不能为 0，无法计算单价"));
        }
        Ok(())
    }

    fn validate_created_by(&self, created_by: Option<i32>) -> Result<i32, AppError> {
        created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))
    }

    fn build_adjustment_summary(
        &self,
        product_name: String,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        batch_no: &str,
        color_no: &str,
        warehouse_name: String,
    ) -> String {
        format!(
            "库存调整：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no, warehouse_name
        )
    }

    fn build_overage_voucher_request(&self, args: &BridgeVoucherArgs) -> CreateVoucherRequest {
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: args.amount,
                    credit: Decimal::ZERO,
                    summary: Some(args.summary.to_string()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some((args.amount / args.quantity_meters).round_dp(2)),
                }),
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1901",
                    subject_name: "待处理财产损溢",
                    debit: Decimal::ZERO,
                    credit: args.amount,
                    summary: Some(args.summary.to_string()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
            ],
        }
    }

    fn build_shortage_voucher_request(&self, args: &BridgeVoucherArgs) -> CreateVoucherRequest {
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1901",
                    subject_name: "待处理财产损溢",
                    debit: args.amount,
                    credit: Decimal::ZERO,
                    summary: Some(args.summary.to_string()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: Decimal::ZERO,
                    credit: args.amount,
                    summary: Some(args.summary.to_string()),
                    quantity_meters: Some(-args.quantity_meters),
                    quantity_kg: Some(-args.quantity_kg),
                    unit_price: Some((args.amount / (-args.quantity_meters)).round_dp(2)),
                }),
            ],
        }
    }

    async fn create_and_log_voucher(
        &self,
        voucher_request: CreateVoucherRequest,
        user_id: i32,
        batch_no: &str,
        color_no: &str,
    ) -> Result<(), AppError> {
        let voucher_service = VoucherService::new(self.db.clone());
        let voucher = voucher_service.create_and_post(voucher_request, user_id).await?;

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
        let voucher_request = Self::build_receipt_voucher_request(BridgeVoucherArgs {
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            summary: &summary,
            amount,
            quantity_meters,
            quantity_kg,
        });
        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = Self::resolve_voucher_user_id(created_by)?;
        // 批次 356 v13 复审 F-P0-2 修复：create → create_and_post 自动过账，触发科目余额回写
        let voucher = voucher_service
            .create_and_post(voucher_request, user_id)
            .await?;
        info!(
            "自动生成生产入库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );
        Ok(())
    }

    /// 构建生产入库凭证请求（借库存商品 / 贷生产成本）
    fn build_receipt_voucher_request(args: BridgeVoucherArgs<'_>) -> CreateVoucherRequest {
        let BridgeVoucherArgs {
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            summary,
            amount,
            quantity_meters,
            quantity_kg,
        } = args;
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
                // 借：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.to_string()),
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
                    summary: Some(summary.to_string()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
            ],
        }
    }

    /// 解析凭证创建用户 ID，缺失时拒绝生成凭证
    fn resolve_voucher_user_id(created_by: Option<i32>) -> Result<i32, AppError> {
        created_by.ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))
    }

    /// 获取生产领料凭证上下文：产品名+成本价+仓库名
    async fn fetch_production_issue_context(
        &self,
        product_id: i32,
        warehouse_id: i32,
    ) -> ProductionIssueVoucherContext {
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));
        ProductionIssueVoucherContext {
            product_name,
            warehouse_name,
            cost_price,
        }
    }

    /// 构造生产领料凭证请求：借生产成本/贷库存商品
    fn build_production_issue_voucher_request(
        args: &VoucherCreateArgs<'_>,
        ctx: &ProductionIssueVoucherContext,
    ) -> CreateVoucherRequest {
        let summary = format!(
            "生产领料：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            ctx.product_name, args.quantity_meters, args.quantity_kg,
            args.batch_no, args.color_no, ctx.warehouse_name
        );
        let amount = (ctx.cost_price * args.quantity_meters).round_dp(2);
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                // 借：生产成本
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "5001",
                    subject_name: "生产成本",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some((amount / args.quantity_meters).round_dp(2)),
                }),
                // 贷：库存商品
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some((amount / args.quantity_meters).round_dp(2)),
                }),
            ],
        }
    }

    /// 创建生产领料凭证：借生产成本/贷库存商品
    async fn create_production_issue_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        // P0 5-4 修复：除零保护，quantity_meters 为 0 时拒绝生成凭证
        if args.quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        let ctx = self
            .fetch_production_issue_context(args.product_id, args.warehouse_id)
            .await;
        let voucher_request = Self::build_production_issue_voucher_request(&args, &ctx);
        let user_id = Self::resolve_voucher_user_id(args.created_by)?;
        let voucher_service = VoucherService::new(self.db.clone());
        // 批次 356 v13 复审 F-P0-2 修复：create → create_and_post 自动过账
        let voucher = voucher_service.create_and_post(voucher_request, user_id).await?;
        info!(
            "自动生成生产领料凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, args.batch_no, args.color_no
        );
        Ok(())
    }

    /// 获取采购退货凭证构造上下文：产品名+成本价+仓库名
    async fn fetch_purchase_return_context(
        &self,
        product_id: i32,
        warehouse_id: i32,
    ) -> PurchaseReturnVoucherContext {
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));
        PurchaseReturnVoucherContext {
            product_name,
            warehouse_name,
            cost_price,
        }
    }

    /// 构造采购退货凭证请求：借应付账款（红字）/贷库存商品（红字）
    fn build_purchase_return_voucher_request(
        args: &VoucherCreateArgs<'_>,
        ctx: &PurchaseReturnVoucherContext,
    ) -> CreateVoucherRequest {
        let summary = format!(
            "采购退货：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            ctx.product_name, args.quantity_meters, args.quantity_kg,
            args.batch_no, args.color_no, ctx.warehouse_name
        );
        let amount = (ctx.cost_price * args.quantity_meters.abs()).round_dp(2);
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                // 借：应付账款（红字）
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "2202",
                    subject_name: "应付账款",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
                // 贷：库存商品（红字）
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some((amount / args.quantity_meters).round_dp(2)),
                }),
            ],
        }
    }

    /// 创建采购退货凭证
    /// 借：应付账款（红字）
    /// 贷：库存商品（红字）
    ///
    /// 批次 356 v13 复审 B-P0-5 修复：采购退货财务凭证未生成
    /// 采购退货业务流闭环：purchase_return_service → 发布 PURCHASE_RETURN 事件 →
    /// inventory_finance_bridge_service 生成红字凭证
    async fn create_purchase_return_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        if args.quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        let ctx = self
            .fetch_purchase_return_context(args.product_id, args.warehouse_id)
            .await;
        let voucher_request = Self::build_purchase_return_voucher_request(&args, &ctx);
        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = args
            .created_by
            .ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service
            .create_and_post(voucher_request, user_id)
            .await?;
        info!(
            "自动生成采购退货凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, args.batch_no, args.color_no
        );
        Ok(())
    }

    /// 获取销售退货凭证构造上下文（产品名/仓库名/成本价，失败降级）
    async fn fetch_sales_return_context(
        &self,
        product_id: i32,
        warehouse_id: i32,
    ) -> SalesReturnVoucherContext {
        let (product_name, cost_price) = self
            .get_product_info(product_id)
            .await
            .unwrap_or_else(|_| (format!("产品{}", product_id), Decimal::ZERO));
        let warehouse_name = self
            .get_warehouse_name(warehouse_id)
            .await
            .unwrap_or_else(|_| format!("仓库{}", warehouse_id));
        SalesReturnVoucherContext {
            product_name,
            warehouse_name,
            cost_price,
        }
    }

    /// 构建销售退货凭证请求（借 1405 库存商品 / 贷 6401 主营业务成本）
    fn build_sales_return_voucher_request(
        args: &VoucherCreateArgs<'_>,
        ctx: &SalesReturnVoucherContext,
    ) -> CreateVoucherRequest {
        let summary = format!(
            "销售退货：{} {}米 {}公斤 批次:{} 色号:{} 仓库:{}",
            ctx.product_name, args.quantity_meters, args.quantity_kg,
            args.batch_no, args.color_no, ctx.warehouse_name
        );
        let amount = (ctx.cost_price * args.quantity_meters.abs()).round_dp(2);
        CreateVoucherRequest {
            voucher_type: "记".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: args.source_bill_type.map(|s| s.to_string()),
            source_module: Some("inventory".to_string()),
            source_bill_id: args.source_bill_id,
            source_bill_no: args.source_bill_no.map(|s| s.to_string()),
            batch_no: Some(args.batch_no.to_string()),
            color_no: Some(args.color_no.to_string()),
            items: vec![
                // 借：库存商品（退货入库）
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 1,
                    subject_code: "1405",
                    subject_name: "库存商品",
                    debit: amount,
                    credit: Decimal::ZERO,
                    summary: Some(summary.clone()),
                    quantity_meters: Some(args.quantity_meters),
                    quantity_kg: Some(args.quantity_kg),
                    unit_price: Some((amount / args.quantity_meters).round_dp(2)),
                }),
                // 贷：主营业务成本（红字反转）
                Self::make_voucher_item(VoucherItemArgs {
                    line_no: 2,
                    subject_code: "6401",
                    subject_name: "主营业务成本",
                    debit: Decimal::ZERO,
                    credit: amount,
                    summary: Some(summary.clone()),
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                }),
            ],
        }
    }

    /// 创建销售退货凭证
    /// 借：库存商品
    /// 贷：主营业务成本（红字反转）
    ///
    /// 批次 356 v13 复审 B-P0-6 修复：销售退货财务凭证未生成
    /// 销售退货业务流闭环：sales_return_service → 发布 SALES_RETURN 事件 →
    /// inventory_finance_bridge_service 生成红字凭证
    async fn create_sales_return_voucher(
        &self,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        if args.quantity_meters.is_zero() {
            return Err(AppError::validation(
                "quantity_meters 不能为 0，无法计算单价",
            ));
        }
        let ctx = self
            .fetch_sales_return_context(args.product_id, args.warehouse_id)
            .await;
        let voucher_request = Self::build_sales_return_voucher_request(&args, &ctx);
        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = args
            .created_by
            .ok_or_else(|| AppError::validation("缺少创建用户ID，无法生成财务凭证"))?;
        let voucher = voucher_service
            .create_and_post(voucher_request, user_id)
            .await?;
        info!(
            "自动生成销售退货凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, args.batch_no, args.color_no
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
