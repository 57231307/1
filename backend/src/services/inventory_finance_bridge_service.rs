#![allow(dead_code)]

use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::voucher_service::{CreateVoucherRequest, VoucherItemRequest, VoucherService};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tracing::{info, error};

/// 库存财务桥接服务
/// 负责监听库存变动事件并自动生成相应的会计凭证
pub struct InventoryFinanceBridgeService {
    db: Arc<DatabaseConnection>,
}

impl InventoryFinanceBridgeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建默认凭证分录（填充所有必填字段）
    #[allow(clippy::too_many_arguments)]
    fn make_voucher_item(
        line_no: i32,
        subject_code: &str,
        subject_name: &str,
        debit: Decimal,
        credit: Decimal,
        summary: Option<String>,
        quantity_meters: Option<Decimal>,
        quantity_kg: Option<Decimal>,
        unit_price: Option<Decimal>,
    ) -> VoucherItemRequest {
        VoucherItemRequest {
            line_no: Some(line_no),
            subject_code: Some(subject_code.to_string()),
            subject_name: Some(subject_name.to_string()),
            debit,
            credit,
            summary,
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
            quantity_meters,
            quantity_kg,
            unit_price,
        }
    }

    /// 启动库存变动事件监听器
    pub fn start_listener(db: Arc<DatabaseConnection>) {
        let mut receiver = EVENT_BUS.subscribe();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
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
                } = event {
                    info!(
                        "处理库存交易创建事件: 交易ID={}, 类型={}, 产品ID={}, 仓库ID={}",
                        transaction_id, transaction_type, product_id, warehouse_id
                    );

                    let bridge_service = InventoryFinanceBridgeService::new(db.clone());
                    if let Err(e) = bridge_service.handle_inventory_transaction(
                        transaction_id,
                        &transaction_type,
                        product_id,
                        warehouse_id,
                        quantity_meters,
                        quantity_kg,
                        source_bill_type.as_deref(),
                        source_bill_no.as_deref(),
                        source_bill_id,
                        &batch_no,
                        &color_no,
                        created_by,
                    ).await {
                        error!("处理库存交易事件失败: 交易ID={}, 错误={}", transaction_id, e);
                    }
                }
            }
        });
    }

    /// 处理库存交易事件，生成相应的会计凭证
    #[allow(clippy::too_many_arguments)]
    async fn handle_inventory_transaction(
        &self,
        _transaction_id: i32,
        transaction_type: &str,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<&str>,
        source_bill_no: Option<&str>,
        source_bill_id: Option<i32>,
        batch_no: &str,
        color_no: &str,
        created_by: Option<i32>,
    ) -> Result<(), AppError> {
        // 根据交易类型生成不同的凭证
        match transaction_type {
            "PURCHASE_RECEIPT" => {
                // 采购入库凭证：借：库存商品 / 贷：应付账款
                self.create_purchase_receipt_voucher(
                    product_id, warehouse_id, quantity_meters, quantity_kg,
                    source_bill_type, source_bill_no, source_bill_id,
                    batch_no, color_no, created_by,
                ).await?;
            }
            "SALES_DELIVERY" => {
                // 销售出库凭证：借：主营业务成本 / 贷：库存商品
                self.create_sales_delivery_voucher(
                    product_id, warehouse_id, quantity_meters, quantity_kg,
                    source_bill_type, source_bill_no, source_bill_id,
                    batch_no, color_no, created_by,
                ).await?;
            }
            "INVENTORY_ADJUSTMENT" => {
                // 库存调整凭证
                self.create_inventory_adjustment_voucher(
                    product_id, warehouse_id, quantity_meters, quantity_kg,
                    source_bill_type, source_bill_no, source_bill_id,
                    batch_no, color_no, created_by,
                ).await?;
            }
            "PRODUCTION_RECEIPT" => {
                // 生产入库凭证：借：库存商品 / 贷：生产成本
                self.create_production_receipt_voucher(
                    product_id, warehouse_id, quantity_meters, quantity_kg,
                    source_bill_type, source_bill_no, source_bill_id,
                    batch_no, color_no, created_by,
                ).await?;
            }
            "PRODUCTION_ISSUE" => {
                // 生产领料凭证：借：生产成本 / 贷：库存商品
                self.create_production_issue_voucher(
                    product_id, warehouse_id, quantity_meters, quantity_kg,
                    source_bill_type, source_bill_no, source_bill_id,
                    batch_no, color_no, created_by,
                ).await?;
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
    #[allow(clippy::too_many_arguments)]
    async fn create_purchase_receipt_voucher(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<&str>,
        source_bill_no: Option<&str>,
        source_bill_id: Option<i32>,
        batch_no: &str,
        color_no: &str,
        created_by: Option<i32>,
    ) -> Result<(), AppError> {
        let product_name = self.get_product_name(product_id).await.unwrap_or_else(|_| format!("产品{}", product_id));
        let _ = self.get_warehouse_name(warehouse_id).await;

        let summary = format!("采购入库：{} {}米 {}公斤 批次:{} 色号:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no);

        let amount = self.calculate_inventory_amount(product_id, quantity_meters).await?;

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
                Self::make_voucher_item(
                    1, "1405", "库存商品", amount, Decimal::ZERO,
                    Some(summary.clone()),
                    Some(quantity_meters), Some(quantity_kg),
                    Some(amount / quantity_meters),
                ),
                // 贷：应付账款
                Self::make_voucher_item(
                    2, "2202", "应付账款", Decimal::ZERO, amount,
                    Some(summary.clone()),
                    None, None, None,
                ),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = created_by.unwrap_or(0);
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
    #[allow(clippy::too_many_arguments)]
    async fn create_sales_delivery_voucher(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<&str>,
        source_bill_no: Option<&str>,
        source_bill_id: Option<i32>,
        batch_no: &str,
        color_no: &str,
        created_by: Option<i32>,
    ) -> Result<(), AppError> {
        let product_name = self.get_product_name(product_id).await.unwrap_or_else(|_| format!("产品{}", product_id));
        let _ = self.get_warehouse_name(warehouse_id).await;

        let summary = format!("销售出库：{} {}米 {}公斤 批次:{} 色号:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no);

        let amount = self.calculate_inventory_amount(product_id, quantity_meters).await?;

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
                Self::make_voucher_item(
                    1, "6401", "主营业务成本", amount, Decimal::ZERO,
                    Some(summary.clone()),
                    Some(quantity_meters), Some(quantity_kg),
                    Some(amount / quantity_meters),
                ),
                // 贷：库存商品
                Self::make_voucher_item(
                    2, "1405", "库存商品", Decimal::ZERO, amount,
                    Some(summary.clone()),
                    Some(quantity_meters), Some(quantity_kg),
                    Some(amount / quantity_meters),
                ),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = created_by.unwrap_or(0);
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
    #[allow(clippy::too_many_arguments)]
    async fn create_inventory_adjustment_voucher(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<&str>,
        source_bill_no: Option<&str>,
        source_bill_id: Option<i32>,
        batch_no: &str,
        color_no: &str,
        created_by: Option<i32>,
    ) -> Result<(), AppError> {
        let product_name = self.get_product_name(product_id).await.unwrap_or_else(|_| format!("产品{}", product_id));
        let _ = self.get_warehouse_name(warehouse_id).await;

        let summary = format!("库存调整：{} {}米 {}公斤 批次:{} 色号:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no);

        let amount = self.calculate_inventory_amount(product_id, quantity_meters.abs()).await?;

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
                    Self::make_voucher_item(
                        1, "1405", "库存商品", amount, Decimal::ZERO,
                        Some(summary.clone()),
                        Some(quantity_meters), Some(quantity_kg),
                        Some(amount / quantity_meters),
                    ),
                    Self::make_voucher_item(
                        2, "1901", "待处理财产损溢", Decimal::ZERO, amount,
                        Some(summary.clone()),
                        None, None, None,
                    ),
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
                    Self::make_voucher_item(
                        1, "1901", "待处理财产损溢", amount, Decimal::ZERO,
                        Some(summary.clone()),
                        None, None, None,
                    ),
                    Self::make_voucher_item(
                        2, "1405", "库存商品", Decimal::ZERO, amount,
                        Some(summary.clone()),
                        Some(-quantity_meters), Some(-quantity_kg),
                        Some(amount / (-quantity_meters)),
                    ),
                ],
            }
        };

        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = created_by.unwrap_or(0);
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成库存调整凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 创建生产入库凭证
    /// 借：库存商品 / 贷：生产成本
    #[allow(clippy::too_many_arguments)]
    async fn create_production_receipt_voucher(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<&str>,
        source_bill_no: Option<&str>,
        source_bill_id: Option<i32>,
        batch_no: &str,
        color_no: &str,
        created_by: Option<i32>,
    ) -> Result<(), AppError> {
        let product_name = self.get_product_name(product_id).await.unwrap_or_else(|_| format!("产品{}", product_id));
        let _ = self.get_warehouse_name(warehouse_id).await;

        let summary = format!("生产入库：{} {}米 {}公斤 批次:{} 色号:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no);

        let amount = self.calculate_inventory_amount(product_id, quantity_meters).await?;

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
                Self::make_voucher_item(
                    1, "1405", "库存商品", amount, Decimal::ZERO,
                    Some(summary.clone()),
                    Some(quantity_meters), Some(quantity_kg),
                    Some(amount / quantity_meters),
                ),
                // 贷：生产成本
                Self::make_voucher_item(
                    2, "5001", "生产成本", Decimal::ZERO, amount,
                    Some(summary.clone()),
                    None, None, None,
                ),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = created_by.unwrap_or(0);
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成生产入库凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 创建生产领料凭证
    /// 借：生产成本 / 贷：库存商品
    #[allow(clippy::too_many_arguments)]
    async fn create_production_issue_voucher(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<&str>,
        source_bill_no: Option<&str>,
        source_bill_id: Option<i32>,
        batch_no: &str,
        color_no: &str,
        created_by: Option<i32>,
    ) -> Result<(), AppError> {
        let product_name = self.get_product_name(product_id).await.unwrap_or_else(|_| format!("产品{}", product_id));
        let _ = self.get_warehouse_name(warehouse_id).await;

        let summary = format!("生产领料：{} {}米 {}公斤 批次:{} 色号:{}",
            product_name, quantity_meters, quantity_kg, batch_no, color_no);

        let amount = self.calculate_inventory_amount(product_id, quantity_meters).await?;

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
                Self::make_voucher_item(
                    1, "5001", "生产成本", amount, Decimal::ZERO,
                    Some(summary.clone()),
                    Some(quantity_meters), Some(quantity_kg),
                    Some(amount / quantity_meters),
                ),
                // 贷：库存商品
                Self::make_voucher_item(
                    2, "1405", "库存商品", Decimal::ZERO, amount,
                    Some(summary.clone()),
                    Some(quantity_meters), Some(quantity_kg),
                    Some(amount / quantity_meters),
                ),
            ],
        };

        let voucher_service = VoucherService::new(self.db.clone());
        let user_id = created_by.unwrap_or(0);
        let voucher = voucher_service.create(voucher_request, user_id).await?;

        info!(
            "自动生成生产领料凭证: 凭证号={}, 交易关联: 批次={}, 色号={}",
            voucher.voucher_no, batch_no, color_no
        );

        Ok(())
    }

    /// 获取产品名称
    async fn get_product_name(&self, product_id: i32) -> Result<String, AppError> {
        use crate::models::product;
        use sea_orm::EntityTrait;

        let product = product::Entity::find_by_id(product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("产品不存在: {}", product_id)))?;

        Ok(product.name)
    }

    /// 获取仓库名称
    async fn get_warehouse_name(&self, warehouse_id: i32) -> Result<String, AppError> {
        use crate::models::warehouse;
        use sea_orm::EntityTrait;

        let warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("仓库不存在: {}", warehouse_id)))?;

        Ok(warehouse.name)
    }

    /// 计算库存金额（使用产品成本价 x 数量）
    async fn calculate_inventory_amount(
        &self,
        product_id: i32,
        quantity_meters: Decimal,
    ) -> Result<Decimal, AppError> {
        let unit_price = self.get_product_cost_price(product_id).await?;
        Ok(unit_price * quantity_meters)
    }

    /// 获取产品成本价
    async fn get_product_cost_price(&self, product_id: i32) -> Result<Decimal, AppError> {
        use crate::models::product;
        use sea_orm::EntityTrait;

        let product = product::Entity::find_by_id(product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("产品不存在: {}", product_id)))?;

        Ok(product.cost_price.unwrap_or(Decimal::ZERO))
    }
}