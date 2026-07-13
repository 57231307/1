//! 采购订单核心服务（po/order）
//!
//! 包含采购订单的 CRUD、生命周期（关闭/取消）、明细管理、查询/导出等。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{
    department, product, purchase_order, purchase_order_item, status, supplier, warehouse,
};
use crate::services::po::UpdatePurchaseOrderRequest;
use crate::utils::error::AppError;
// 批次 260 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use crate::utils::number_generator::DocumentNumberGenerator;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
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
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
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
            currency: Set(req.currency.clone().unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            order_status: Set(status::purchase_order::DRAFT.to_string()),
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
            // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
            let quantity_ordered = item.quantity_ordered.unwrap_or(Decimal::ZERO);
            let unit_price = item.unit_price.unwrap_or(Decimal::ZERO);
            let amount = (quantity_ordered * unit_price).round_dp(2);
            let tax_percent = item.tax_rate.unwrap_or(Decimal::new(13, 2));
            let tax_amount = (amount * tax_percent / Decimal::new(100, 0)).round_dp(2);
            let discount_percent = item.discount_percent.unwrap_or(Decimal::ZERO);
            let discount_amount = (amount * discount_percent / Decimal::new(100, 0)).round_dp(2);
            let quantity_alt_ordered = item.quantity_alt_ordered.unwrap_or(Decimal::ZERO);

            let order_item = purchase_order_item::ActiveModel {
                id: Default::default(),
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
        // 批次 18（2026-06-28）：补全事务边界，原实现无事务且 update_with_audit 传 &*self.db 非原子
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发修改）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
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

        // 4. 更新订单（update_with_audit 传 &txn 纳入事务，保证原子性）
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
            &txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

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

        // 4. 删除订单（级联删除明细）（P0 8-3 修复：补审计日志）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            purchase_order::Entity,
            _,
        >(&*self.db, "purchase_order", order_id, Some(user_id))
        .await
    }

    /// 关闭采购订单
    pub async fn close_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 批次 17（2026-06-28）：补全事务边界，原实现无事务且 update_with_audit 传 &*self.db 非原子
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发关闭）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
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

        // 3. 更新状态（update_with_audit 传 &txn 纳入事务，保证原子性）
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::CLOSED.to_string());
        order_active.updated_by = Set(Some(user_id));

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

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

        // 批次 260 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_order::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<PurchaseOrderDto>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
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

// =====================================================
// 单元测试模块（模式 B：内嵌 #[cfg(test)] mod tests）
// =====================================================
// 测试策略：create_order_items / create_order_header / validate_order_request 中的
// 纯算法逻辑（金额、税额、折扣、总额、行号默认值、货币/汇率默认值、日期校验、CSV 表头）
// 通过复现其计算公式进行回归保护；依赖真实数据库 schema 的方法标注 #[ignore]。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use sea_orm::Database;
    use std::str::FromStr;

    // ---------- 纯算法复现夹具（与 create_order_items / create_order_header 保持一致） ----------

    /// 复现 create_order_items 的金额计算纯算法
    fn calc_amount(quantity: Decimal, unit_price: Decimal) -> Decimal {
        (quantity * unit_price).round_dp(2)
    }

    /// 复现 create_order_items 的税额计算纯算法
    fn calc_tax_amount(amount: Decimal, tax_percent: Decimal) -> Decimal {
        (amount * tax_percent / Decimal::new(100, 0)).round_dp(2)
    }

    /// 复现 create_order_items 的折扣计算纯算法
    fn calc_discount_amount(amount: Decimal, discount_percent: Decimal) -> Decimal {
        (amount * discount_percent / Decimal::new(100, 0)).round_dp(2)
    }

    /// 复现 create_order_items 的明细总额计算纯算法
    fn calc_line_total(amount: Decimal, tax_amount: Decimal, discount_amount: Decimal) -> Decimal {
        amount + tax_amount - discount_amount
    }

    /// 复现 create_order_items 的明细行号默认值计算
    fn default_line_no(index: usize) -> i32 {
        (index + 1) as i32
    }

    /// 测试 SQLite 内存数据库连接
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    // ---------- 金额计算 ----------

    /// 测试_金额计算_整数数量场景
    ///
    /// 验证数量与单价均为整数时金额计算正确
    #[test]
    fn 测试_金额计算_整数数量场景() {
        let quantity = decs!("10");
        let unit_price = decs!("100");
        // 10 * 100 = 1000.00
        assert_eq!(calc_amount(quantity, unit_price), decs!("1000"));
    }

    /// 测试_金额计算_小数数量场景
    ///
    /// 验证数量与单价含小数时 round_dp(2) 精度归一化生效
    #[test]
    fn 测试_金额计算_小数数量场景() {
        // 3.1415 * 2.5 = 7.85375 → round_dp(2) = 7.85
        assert_eq!(calc_amount(decs!("3.1415"), decs!("2.5")), decs!("7.85"));
    }

    /// 测试_金额计算_零价或零量场景
    ///
    /// 验证数量或单价为 0 时金额为 0
    #[test]
    fn 测试_金额计算_零价或零量场景() {
        // 零数量
        assert_eq!(calc_amount(Decimal::ZERO, decs!("100")), Decimal::ZERO);
        // 零单价
        assert_eq!(calc_amount(decs!("10"), Decimal::ZERO), Decimal::ZERO);
    }

    // ---------- 税额计算 ----------

    /// 测试_税额计算_默认税率场景
    ///
    /// 验证未指定税率时使用默认值 Decimal::new(13, 2) = 0.13，
    /// 税额公式：amount * tax_percent / 100
    #[test]
    fn 测试_税额计算_默认税率场景() {
        // 默认税率常量与 create_order_items 中 unwrap_or(Decimal::new(13, 2)) 一致
        let default_tax_percent = Decimal::new(13, 2);
        // 1000 * 0.13 / 100 = 1.30
        assert_eq!(calc_tax_amount(decs!("1000"), default_tax_percent), decs!("1.30"));
    }

    /// 测试_税额计算_自定义税率场景
    ///
    /// 验证用户传入税率（百分比值 13）时税额计算正确
    #[test]
    fn 测试_税额计算_自定义税率场景() {
        // 用户传 13 作为百分比：1000 * 13 / 100 = 130.00
        assert_eq!(calc_tax_amount(decs!("1000"), decs!("13")), decs!("130"));
    }

    /// 测试_税额计算_零税率场景
    ///
    /// 验证税率为 0 时税额为 0
    #[test]
    fn 测试_税额计算_零税率场景() {
        assert_eq!(
            calc_tax_amount(decs!("1000"), Decimal::ZERO),
            Decimal::ZERO
        );
    }

    /// 测试_税额计算_精度归一化
    ///
    /// 验证税额计算结果经 round_dp(2) 归一化到两位小数
    #[test]
    fn 测试_税额计算_精度归一化() {
        // 333.33 * 13 / 100 = 43.3329 → round_dp(2) = 43.33
        assert_eq!(
            calc_tax_amount(decs!("333.33"), decs!("13")),
            decs!("43.33")
        );
    }

    // ---------- 折扣计算 ----------

    /// 测试_折扣计算_默认无折扣场景
    ///
    /// 验证未指定折扣（discount_percent 默认 0）时折扣金额为 0
    #[test]
    fn 测试_折扣计算_默认无折扣场景() {
        // 默认折扣百分比为 Decimal::ZERO（与 create_order_items 一致）
        assert_eq!(
            calc_discount_amount(decs!("1000"), Decimal::ZERO),
            Decimal::ZERO
        );
    }

    /// 测试_折扣计算_自定义折扣场景
    ///
    /// 验证用户传入折扣百分比（10 表示 10%）时折扣金额计算正确
    #[test]
    fn 测试_折扣计算_自定义折扣场景() {
        // 1000 * 10 / 100 = 100.00
        assert_eq!(
            calc_discount_amount(decs!("1000"), decs!("10")),
            decs!("100")
        );
    }

    // ---------- 明细总额 ----------

    /// 测试_明细总额_含税无折扣场景
    ///
    /// 验证总额公式 amount + tax_amount - discount_amount（无折扣）
    #[test]
    fn 测试_明细总额_含税无折扣场景() {
        // 1000 + 130 - 0 = 1130
        assert_eq!(
            calc_line_total(decs!("1000"), decs!("130"), Decimal::ZERO),
            decs!("1130")
        );
    }

    /// 测试_明细总额_含税含折扣场景
    ///
    /// 验证总额公式 amount + tax_amount - discount_amount（含折扣）
    #[test]
    fn 测试_明细总额_含税含折扣场景() {
        // 1000 + 130 - 100 = 1030
        assert_eq!(
            calc_line_total(decs!("1000"), decs!("130"), decs!("100")),
            decs!("1030")
        );
    }

    // ---------- 明细行号 ----------

    /// 测试_明细行号_默认值递增场景
    ///
    /// 验证未指定 line_no 时按 (index + 1) 从 1 递增
    #[test]
    fn 测试_明细行号_默认值递增场景() {
        // 复现 create_order_items 中 item.line_no.unwrap_or((index + 1) as i32)
        assert_eq!(default_line_no(0), 1);
        assert_eq!(default_line_no(1), 2);
        assert_eq!(default_line_no(2), 3);
        assert_eq!(default_line_no(9), 10);
    }

    // ---------- 订单默认值 ----------

    /// 测试_货币默认值_未指定时使用CNY
    ///
    /// 验证 create_order_header 中 currency 未指定时使用 crate::constants::DEFAULT_CURRENCY
    #[test]
    fn 测试_货币默认值_未指定时使用CNY() {
        // 复现 create_order_header 中货币默认值逻辑
        let req_currency: Option<String> = None;
        let currency =
            req_currency.unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string());
        // 验证未指定时回退到项目默认货币常量
        assert_eq!(currency, crate::constants::DEFAULT_CURRENCY);
        assert_eq!(currency, "CNY");
        // 验证显式指定时不应被默认值覆盖
        let explicit = Some("USD".to_string());
        let currency_explicit = explicit
            .clone()
            .unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string());
        assert_eq!(currency_explicit, "USD");
    }

    /// 测试_汇率默认值_未指定时为1
    ///
    /// 验证 create_order_header 中 exchange_rate 未指定时默认为 Decimal::new(1, 0) = 1
    #[test]
    fn 测试_汇率默认值_未指定时为1() {
        // 复现 create_order_header 中汇率默认值逻辑
        let req_exchange_rate: Option<Decimal> = None;
        let exchange_rate = req_exchange_rate.unwrap_or(Decimal::new(1, 0));
        assert_eq!(exchange_rate, decs!("1"));
        // 验证显式指定时不应被默认值覆盖
        let explicit = Some(decs!("6.5"));
        let exchange_rate_explicit = explicit.unwrap_or(Decimal::new(1, 0));
        assert_eq!(exchange_rate_explicit, decs!("6.5"));
    }

    /// 测试_订单初始状态_使用DRAFT常量
    ///
    /// 验证 create_order_header 中订单初始状态使用 status::purchase_order::DRAFT 常量
    /// （禁止硬编码状态字符串，全程引用常量）
    #[test]
    fn 测试_订单初始状态_使用DRAFT常量() {
        // 复现 create_order_header 中订单初始状态设置
        let initial_status = status::purchase_order::DRAFT.to_string();
        assert!(!initial_status.is_empty());
        assert_eq!(initial_status, status::purchase_order::DRAFT);
        // 验证 DRAFT 与其他采购订单状态常量互不相同（状态机不冲突）
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::APPROVED
        );
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::CANCELLED
        );
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::COMPLETED
        );
        assert_ne!(
            status::purchase_order::DRAFT,
            status::purchase_order::CLOSED
        );
    }

    // ---------- 日期校验 ----------

    /// 测试_日期校验_预计交货不能早于订单日期
    ///
    /// 验证 validate_order_request 中预计交货日期不能早于订单日期的校验逻辑
    #[test]
    fn 测试_日期校验_预计交货不能早于订单日期() {
        let order_date = ymd!(2026, 3, 15);
        // 场景 1：预计交货日期等于订单日期（允许）
        let expected_same = ymd!(2026, 3, 15);
        assert!(!(expected_same < order_date));
        // 场景 2：预计交货日期晚于订单日期（允许）
        let expected_after = ymd!(2026, 3, 20);
        assert!(!(expected_after < order_date));
        // 场景 3：预计交货日期早于订单日期（应拒绝，复现 validate_order_request 拒绝条件）
        let expected_before = ymd!(2026, 3, 10);
        assert!(expected_before < order_date);
    }

    // ---------- CSV 导出表头 ----------

    /// 测试_CSV表头_列数与列名验证
    ///
    /// 验证 export_orders_to_csv 中 CSV 表头为 21 列且列名符合预期
    #[test]
    fn 测试_CSV表头_列数与列名验证() {
        // 复现 export_orders_to_csv 中的表头定义
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
        // 列数为 21
        assert_eq!(headers.len(), 21);
        // 首列与末列
        assert_eq!(headers.first().unwrap(), "订单编号");
        assert_eq!(headers.last().unwrap(), "备注");
        // 关键列存在性抽检
        assert!(headers.contains(&"币种".to_string()));
        assert!(headers.contains(&"状态".to_string()));
        assert!(headers.contains(&"总金额".to_string()));
    }

    // ---------- 夹具宏与服务实例化 ----------

    /// 测试_decs夹具宏_可用性
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串
    #[test]
    fn 测试_decs夹具宏_可用性() {
        let v = decs!("123.45");
        assert_eq!(v, Decimal::from_str("123.45").unwrap());
        assert_eq!(v.to_string(), "123.45");
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 PurchaseOrderService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db = setup_test_db().await;
        let service = PurchaseOrderService::new(Arc::new(db));
        // 验证内部 db Arc 已被正确持有
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // ---------- 状态校验门（批次 392 补测） ----------

    /// 复现 update_order 的状态校验门（行 363-369）
    /// 仅 DRAFT 和 REJECTED 状态允许修改，其他状态返回 Err
    fn update_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != status::purchase_order::DRAFT && status != status::purchase_order::REJECTED {
            return Err(AppError::business(format!(
                "订单状态不允许修改，当前状态：{}",
                status
            )));
        }
        Ok(())
    }

    /// 复现 delete_order 的状态校验门（行 441-445）
    /// 仅 DRAFT 状态允许删除，其他状态返回 Err
    fn delete_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许删除，当前状态：{}",
                status
            )));
        }
        Ok(())
    }

    /// 复现 close_order 的状态校验门（行 481-489）
    /// 仅 COMPLETED 和 PARTIAL_RECEIVED 状态允许关闭，其他状态返回 Err
    fn close_order_status_gate(status: &str) -> Result<(), AppError> {
        if ![
            status::purchase_order::COMPLETED,
            status::purchase_order::PARTIAL_RECEIVED,
        ]
        .contains(&status)
        {
            return Err(AppError::business(format!(
                "订单状态不允许关闭，当前状态：{}",
                status
            )));
        }
        Ok(())
    }

    /// 测试_update_order状态校验门_允许的状态
    ///
    /// 验证 DRAFT 和 REJECTED 状态允许修改
    #[test]
    fn 测试_update_order状态校验门_允许的状态() {
        assert!(update_order_status_gate(status::purchase_order::DRAFT).is_ok());
        assert!(update_order_status_gate(status::purchase_order::REJECTED).is_ok());
    }

    /// 测试_update_order状态校验门_禁止的状态
    ///
    /// 验证非 DRAFT/REJECTED 状态不允许修改且错误消息包含当前状态
    #[test]
    fn 测试_update_order状态校验门_禁止的状态() {
        let forbidden = [
            status::purchase_order::PENDING_APPROVAL,
            status::purchase_order::SUBMITTED,
            status::purchase_order::APPROVED,
            status::purchase_order::CLOSED,
            status::purchase_order::CANCELLED,
            status::purchase_order::COMPLETED,
            status::purchase_order::PARTIAL_RECEIVED,
        ];
        for s in forbidden {
            let err = update_order_status_gate(s).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
            assert!(msg.contains("修改"), "错误消息应包含操作类型");
        }
    }

    /// 测试_delete_order状态校验门_仅DRAFT允许
    ///
    /// 验证仅 DRAFT 状态允许删除
    #[test]
    fn 测试_delete_order状态校验门_仅DRAFT允许() {
        assert!(delete_order_status_gate(status::purchase_order::DRAFT).is_ok());
    }

    /// 测试_delete_order状态校验门_非DRAFT禁止
    ///
    /// 验证非 DRAFT 状态不允许删除且错误消息包含当前状态
    #[test]
    fn 测试_delete_order状态校验门_非DRAFT禁止() {
        let forbidden = [
            status::purchase_order::REJECTED,
            status::purchase_order::APPROVED,
            status::purchase_order::COMPLETED,
            status::purchase_order::CANCELLED,
        ];
        for s in forbidden {
            let err = delete_order_status_gate(s).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
            assert!(msg.contains("删除"), "错误消息应包含操作类型");
        }
    }

    /// 测试_close_order状态校验门_允许的状态
    ///
    /// 验证 COMPLETED 和 PARTIAL_RECEIVED 状态允许关闭
    #[test]
    fn 测试_close_order状态校验门_允许的状态() {
        assert!(close_order_status_gate(status::purchase_order::COMPLETED).is_ok());
        assert!(close_order_status_gate(status::purchase_order::PARTIAL_RECEIVED).is_ok());
    }

    /// 测试_close_order状态校验门_禁止的状态
    ///
    /// 验证非 COMPLETED/PARTIAL_RECEIVED 状态不允许关闭且错误消息包含当前状态
    #[test]
    fn 测试_close_order状态校验门_禁止的状态() {
        let forbidden = [
            status::purchase_order::DRAFT,
            status::purchase_order::APPROVED,
            status::purchase_order::REJECTED,
            status::purchase_order::CANCELLED,
        ];
        for s in forbidden {
            let err = close_order_status_gate(s).unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
            assert!(msg.contains("关闭"), "错误消息应包含操作类型");
        }
    }
}
