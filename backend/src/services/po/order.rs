//! 采购订单核心服务（po/order）
//!
//! 包含采购订单的 CRUD、生命周期（关闭/取消）、明细管理、查询/导出等。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{
    department, product, purchase_order, purchase_order_item, status, supplier, warehouse,
};
use crate::services::po::UpdatePurchaseOrderRequest;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Serialize;
use std::sync::Arc;

// =====================================================
// 响应 DTO
// =====================================================

/// 采购订单视图对象
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderDto {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: chrono::NaiveDate,
    pub expected_delivery_date: Option<chrono::NaiveDate>,
    pub actual_delivery_date: Option<chrono::NaiveDate>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub purchaser_id: i32,
    pub currency: String,
    pub exchange_rate: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub total_amount_foreign: rust_decimal::Decimal,
    pub total_quantity: rust_decimal::Decimal,
    pub total_quantity_alt: rust_decimal::Decimal,
    #[serde(rename = "status")]
    pub order_status: String,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 采购订单明细视图对象
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderItemDto {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    #[serde(rename = "material_id")]
    pub product_id: i32,
    #[serde(rename = "material_code")]
    pub material_code: Option<String>,
    #[serde(rename = "material_name")]
    pub material_name: Option<String>,
    #[serde(rename = "quantity_ordered")]
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    #[serde(rename = "tax_rate")]
    pub tax_percent: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub returned_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
}

// =====================================================
// 采购订单服务
// =====================================================

/// 采购订单服务（核心）
pub struct PurchaseOrderService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl PurchaseOrderService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成采购订单号
    // 格式：PO + 年月日 + 三位序号（PO20260315001）
    crate::impl_generate_no!(
        generate_order_no,
        "PO",
        purchase_order::Entity,
        purchase_order::Column::OrderNo
    );

    // 生成采购订单号（使用事务连接）
    // 格式：PO + 年月日 + 三位序号（PO20260315001）
    crate::impl_generate_no!(
        generate_order_no_with_txn,
        "PO",
        purchase_order::Entity,
        purchase_order::Column::OrderNo,
        txn
    );

    /// 创建采购订单（含明细）
    pub async fn create_order(
        &self,
        req: crate::services::po::CreatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 验证请求参数
        let (warehouse_id, department_id) = self.validate_order_request(&req, &txn).await?;

        // 2. 创建订单主表
        let order = self
            .create_order_header(&req, warehouse_id, department_id, user_id, &txn)
            .await?;

        // 3. 创建订单明细并计算总金额
        let (total_amount, total_quantity, total_quantity_alt) =
            self.create_order_items(&req, order.id, &txn).await?;

        // 4. 更新订单总金额和数量
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        // 5. 预算检查与占用（非阻断）
        self.check_and_occupy_budget(&order, department_id, total_amount, user_id)
            .await;

        // 6. 提交事务
        txn.commit().await?;

        Ok(order)
    }

    /// 验证采购订单请求参数
    async fn validate_order_request(
        &self,
        req: &crate::services::po::CreatePurchaseOrderRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(i32, i32), AppError> {
        // 检查供应商是否存在
        let supplier_exists = supplier::Entity::find_by_id(req.supplier_id)
            .one(txn)
            .await?;
        if supplier_exists.is_none() {
            tracing::error!("供应商 ID {} 不存在", req.supplier_id);
            return Err(AppError::bad_request(format!(
                "供应商 ID {} 不存在",
                req.supplier_id
            )));
        }

        // 检查仓库是否存在
        let warehouse_id = req
            .warehouse_id
            .ok_or_else(|| AppError::bad_request("仓库 ID 不能为空"))?;
        let warehouse_exists = warehouse::Entity::find_by_id(warehouse_id).one(txn).await?;
        if warehouse_exists.is_none() {
            tracing::error!("仓库 ID {} 不存在", warehouse_id);
            return Err(AppError::bad_request(format!(
                "仓库 ID {} 不存在",
                warehouse_id
            )));
        }

        // 检查部门是否存在
        let department_id = req
            .department_id
            .ok_or_else(|| AppError::bad_request("部门 ID 不能为空"))?;
        let department_exists = department::Entity::find_by_id(department_id)
            .one(txn)
            .await?;
        if department_exists.is_none() {
            tracing::error!("Transaction rolled back: 部门 ID {} 不存在", department_id);
            return Err(AppError::bad_request(format!(
                "部门 ID {} 不存在",
                department_id
            )));
        }

        // 日期合理性检查
        if let Some(expected_date) = req.expected_delivery_date {
            if expected_date < req.order_date {
                tracing::error!("预计交货日期不能早于订单日期");
                return Err(AppError::bad_request(
                    "预计交货日期不能早于订单日期".to_string(),
                ));
            }
        }

        Ok((warehouse_id, department_id))
    }

    /// 创建采购订单主表
    async fn create_order_header(
        &self,
        req: &crate::services::po::CreatePurchaseOrderRequest,
        warehouse_id: i32,
        department_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<purchase_order::Model, AppError> {
        // 生成订单号
        let order_no = DocumentNumberGenerator::generate_no(
            txn,
            "PO",
            purchase_order::Entity,
            purchase_order::Column::OrderNo,
        )
        .await?;

        // 创建采购订单主表
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(req.supplier_id),
            order_date: Set(req.order_date),
            expected_delivery_date: Set(req.expected_delivery_date),
            warehouse_id: Set(warehouse_id),
            department_id: Set(department_id),
            purchaser_id: Set(user_id),
            currency: Set(req.currency.clone().unwrap_or_else(|| "CNY".to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            order_status: Set("DRAFT".to_string()),
            payment_terms: Set(req.payment_terms.clone()),
            shipping_terms: Set(req.shipping_terms.clone()),
            notes: Set(req.notes.clone()),
            attachment_urls: Set(req.attachment_urls.clone()),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(txn)
        .await?;

        Ok(order)
    }

    /// 创建采购订单明细
    async fn create_order_items(
        &self,
        req: &crate::services::po::CreatePurchaseOrderRequest,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(Decimal, Decimal, Decimal), AppError> {
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        let items = req.items.clone().unwrap_or_default();

        // 业务验证：产品是否存在（批量查询优化）
        {
            let mut product_ids = std::collections::HashSet::new();
            for item in &items {
                if let Some(material_id) = item.material_id {
                    if material_id != 0 {
                        product_ids.insert(material_id);
                    }
                }
            }
            if !product_ids.is_empty() {
                let existing_products = product::Entity::find()
                    .filter(
                        product::Column::Id.is_in(product_ids.iter().cloned().collect::<Vec<_>>()),
                    )
                    .all(txn)
                    .await?;
                let existing_product_ids: std::collections::HashSet<i32> =
                    existing_products.into_iter().map(|p| p.id).collect();
                for material_id in &product_ids {
                    if !existing_product_ids.contains(material_id) {
                        tracing::error!("Transaction rolled back: 物料 ID {} 不存在", material_id);
                        // 事务将在 ? 传播 Err 时由 DatabaseTransaction 的 Drop 自动回滚
                        return Err(AppError::bad_request(format!(
                            "物料 ID {} 不存在",
                            material_id
                        )));
                    }
                }
            }
        }

        // 创建订单明细
        for (index, item) in items.iter().enumerate() {
            let quantity_ordered = item.quantity_ordered.unwrap_or(Decimal::ZERO);
            let unit_price = item.unit_price.unwrap_or(Decimal::ZERO);
            let amount = quantity_ordered * unit_price;
            let tax_percent = item.tax_rate.unwrap_or(Decimal::new(13, 2));
            let tax_amount = amount * tax_percent / Decimal::new(100, 0);
            let discount_percent = item.discount_percent.unwrap_or(Decimal::ZERO);
            let discount_amount = amount * discount_percent / Decimal::new(100, 0);
            let quantity_alt_ordered = item.quantity_alt_ordered.unwrap_or(Decimal::ZERO);

            let order_item = purchase_order_item::ActiveModel {
                id: Set(0),
                order_id: Set(order_id),
                line_no: Set(item.line_no.unwrap_or((index + 1) as i32)),
                // material_id 缺失时拒绝创建订单行项，避免脏 product_id=0 记录
                product_id: Set(item.material_id.ok_or_else(|| {
                    AppError::validation(format!("订单行 {} 缺少物料ID", index + 1))
                })?),
                quantity: Set(quantity_ordered),
                quantity_alt: Set(quantity_alt_ordered),
                unit_price: Set(unit_price),
                unit_price_foreign: Set(unit_price),
                discount_percent: Set(discount_percent),
                tax_percent: Set(tax_percent),
                subtotal: Set(amount),
                tax_amount: Set(tax_amount),
                discount_amount: Set(discount_amount),
                total_amount: Set(amount + tax_amount - discount_amount),
                received_quantity: Set(Decimal::ZERO),
                received_quantity_alt: Set(Decimal::ZERO),
                notes: Set(item.notes.clone()),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            order_item.insert(txn).await?;

            total_amount += amount + tax_amount - discount_amount;
            total_quantity += quantity_ordered;
            total_quantity_alt += quantity_alt_ordered;
        }

        Ok((total_amount, total_quantity, total_quantity_alt))
    }

    /// 更新采购订单（仅草稿状态）
    pub async fn update_order(
        &self,
        order_id: i32,
        req: UpdatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT
            && order.order_status != status::purchase_order::REJECTED
        {
            return Err(AppError::business(format!(
                "订单状态不允许修改，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的订单".to_string(),
            ));
        }

        // 4. 更新订单
        let mut order_active: purchase_order::ActiveModel = order.into();

        if let Some(supplier_id) = req.supplier_id {
            order_active.supplier_id = Set(supplier_id);
        }
        if let Some(order_date) = req.order_date {
            order_active.order_date = Set(order_date);
        }
        if let Some(expected_delivery_date) = req.expected_delivery_date {
            order_active.expected_delivery_date = Set(Some(expected_delivery_date));
        }
        if let Some(warehouse_id) = req.warehouse_id {
            order_active.warehouse_id = Set(warehouse_id);
        }
        if let Some(department_id) = req.department_id {
            order_active.department_id = Set(department_id);
        }
        if let Some(currency) = req.currency {
            order_active.currency = Set(currency);
        }
        if let Some(exchange_rate) = req.exchange_rate {
            order_active.exchange_rate = Set(exchange_rate);
        }
        if let Some(payment_terms) = req.payment_terms {
            order_active.payment_terms = Set(Some(payment_terms));
        }
        if let Some(shipping_terms) = req.shipping_terms {
            order_active.shipping_terms = Set(Some(shipping_terms));
        }
        if let Some(notes) = req.notes {
            order_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            order_active.attachment_urls = Set(Some(attachment_urls));
        }

        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(Utc::now());

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        Ok(order)
    }

    /// 删除采购订单（仅草稿状态）
    pub async fn delete_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许删除，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的订单".to_string(),
            ));
        }

        // 4. 删除订单（级联删除明细）
        purchase_order::Entity::delete_by_id(order_id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 关闭采购订单
    pub async fn close_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态（已完成或部分入库的订单才能关闭）
        if ![
            status::purchase_order::COMPLETED,
            status::purchase_order::PARTIAL_RECEIVED,
        ]
        .contains(&order.order_status.as_str())
        {
            return Err(AppError::business(format!(
                "订单状态不允许关闭，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::CLOSED.to_string());
        order_active.updated_by = Set(Some(user_id));

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        Ok(order)
    }

    /// 取消采购订单
    pub async fn cancel_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态（只有草稿、待审批、已拒绝的订单可以取消）
        if ![
            status::purchase_order::DRAFT,
            status::purchase_order::PENDING_APPROVAL,
            status::purchase_order::REJECTED,
        ]
        .contains(&order.order_status.as_str())
        {
            return Err(AppError::business(format!(
                "订单状态不允许取消，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::CANCELLED.to_string());
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(Utc::now());

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        Ok(order)
    }

    /// 获取订单列表（分页）
    pub async fn list_orders(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<PurchaseOrderDto>, u64), AppError> {
        use sea_orm::{JoinType, QuerySelect, RelationTrait};
        let mut query = purchase_order::Entity::find()
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Warehouse.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Department.def(),
            );

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_order::Column::OrderStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_order::Column::SupplierId.eq(supplier_id));
        }

        // 分页查询
        let paginator = query
            .order_by(purchase_order::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<PurchaseOrderDto>()
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 获取订单详情
    pub async fn get_order(&self, order_id: i32) -> Result<PurchaseOrderDto, AppError> {
        use sea_orm::{JoinType, QuerySelect, RelationTrait};
        let order = purchase_order::Entity::find_by_id(order_id)
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(warehouse::Column::Name, "warehouse_name")
            .column_as(department::Column::Name, "department_name")
            .join(JoinType::LeftJoin, purchase_order::Relation::Supplier.def())
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Warehouse.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_order::Relation::Department.def(),
            )
            .into_model::<PurchaseOrderDto>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        Ok(order)
    }

    /// 获取订单明细列表
    pub async fn list_order_items(
        &self,
        order_id: i32,
    ) -> Result<Vec<PurchaseOrderItemDto>, AppError> {
        use sea_orm::{JoinType, QuerySelect, RelationTrait};
        let items = purchase_order_item::Entity::find()
            .column_as(product::Column::Code, "material_code")
            .column_as(product::Column::Name, "material_name")
            .join(
                JoinType::LeftJoin,
                purchase_order_item::Relation::Product.def(),
            )
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .into_model::<PurchaseOrderItemDto>()
            .all(&*self.db)
            .await?;

        Ok(items)
    }

    // ========== 数据导出方法 ==========

    /// 导出采购订单为 CSV 格式
    pub async fn export_orders_to_csv(
        &self,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<Vec<u8>, AppError> {
        let (orders, _total) = self.list_orders(1, 10000, status, supplier_id).await?;

        let headers = vec![
            "订单编号".to_string(),
            "供应商ID".to_string(),
            "供应商名称".to_string(),
            "订单日期".to_string(),
            "预计交货日期".to_string(),
            "实际交货日期".to_string(),
            "仓库ID".to_string(),
            "仓库名称".to_string(),
            "部门ID".to_string(),
            "部门名称".to_string(),
            "采购员ID".to_string(),
            "币种".to_string(),
            "汇率".to_string(),
            "总金额".to_string(),
            "总金额外币".to_string(),
            "总数量".to_string(),
            "总数量辅助".to_string(),
            "状态".to_string(),
            "付款条件".to_string(),
            "运输条款".to_string(),
            "备注".to_string(),
        ];

        let rows: Vec<std::collections::HashMap<String, String>> = orders
            .into_iter()
            .map(|o| {
                let mut row = std::collections::HashMap::new();
                row.insert("订单编号".to_string(), o.order_no);
                row.insert("供应商ID".to_string(), o.supplier_id.to_string());
                row.insert(
                    "供应商名称".to_string(),
                    o.supplier_name.unwrap_or_default(),
                );
                row.insert("订单日期".to_string(), o.order_date.to_string());
                row.insert(
                    "预计交货日期".to_string(),
                    o.expected_delivery_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                );
                row.insert(
                    "实际交货日期".to_string(),
                    o.actual_delivery_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                );
                row.insert("仓库ID".to_string(), o.warehouse_id.to_string());
                row.insert("仓库名称".to_string(), o.warehouse_name.unwrap_or_default());
                row.insert("部门ID".to_string(), o.department_id.to_string());
                row.insert(
                    "部门名称".to_string(),
                    o.department_name.unwrap_or_default(),
                );
                row.insert("采购员ID".to_string(), o.purchaser_id.to_string());
                row.insert("币种".to_string(), o.currency);
                row.insert("汇率".to_string(), o.exchange_rate.to_string());
                row.insert("总金额".to_string(), o.total_amount.to_string());
                row.insert("总金额外币".to_string(), o.total_amount_foreign.to_string());
                row.insert("总数量".to_string(), o.total_quantity.to_string());
                row.insert("总数量辅助".to_string(), o.total_quantity_alt.to_string());
                row.insert("状态".to_string(), o.order_status);
                row.insert("付款条件".to_string(), o.payment_terms.unwrap_or_default());
                row.insert("运输条款".to_string(), o.shipping_terms.unwrap_or_default());
                row.insert("备注".to_string(), o.notes.unwrap_or_default());
                row
            })
            .collect();

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::internal(format!("CSV 生成失败: {}", e)))
    }
}
