//! 采购订单 Service
//!
//! 采购订单服务层，负责采购订单的核心业务逻辑
//! 包含订单创建、审批、执行、退货等全流程管理

use crate::models::{
    department, inventory_stock, product, purchase_order, purchase_order_item, supplier, warehouse,
};
use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, Order,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

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

/// 采购订单服务
pub struct PurchaseOrderService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseOrderService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成采购订单号
    /// 格式：PO + 年月日 + 三位序号（PO20260315001）
    pub async fn generate_order_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("PO{}", today);

        // 查询今日订单数量
        let count = purchase_order::Entity::find()
            .filter(purchase_order::Column::OrderNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;

        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 生成入库单号
    /// 格式：GR + 年月日 + 三位序号（GR20260315001）
    #[allow(dead_code)]
    pub async fn generate_receipt_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("GR{}", today);

        // 查询今日入库单数量
        let count = purchase_order::Entity::find()
            .filter(purchase_order::Column::OrderNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;

        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 创建采购订单（含明细）
    pub async fn create_order(
        &self,
        req: CreatePurchaseOrderRequest,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成订单号
        let order_no = self.generate_order_no().await?;

        // 2. 创建采购订单主表
        let order = purchase_order::ActiveModel {
            order_no: Set(order_no),
            supplier_id: Set(req.supplier_id),
            order_date: Set(req.order_date),
            expected_delivery_date: Set(req.expected_delivery_date),
            warehouse_id: Set(req.warehouse_id),
            department_id: Set(req.department_id),
            purchaser_id: Set(user_id),
            currency: Set(req.currency.unwrap_or_else(|| "CNY".to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            order_status: Set("DRAFT".to_string()),
            payment_terms: Set(req.payment_terms),
            shipping_terms: Set(req.shipping_terms),
            notes: Set(req.notes),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 3. 创建订单明细
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        for item_req in req.items {
            let quantity_ordered = item_req.quantity_ordered;
            let unit_price = item_req.unit_price;
            let amount = quantity_ordered * unit_price;
            total_amount += amount;
            total_quantity += quantity_ordered;

            // 计算辅助数量
            let quantity_alt_ordered = item_req.quantity_alt_ordered.unwrap_or(Decimal::ZERO);
            total_quantity_alt += quantity_alt_ordered;

            // 计算税额和折扣
            let tax_percent = item_req.tax_rate.unwrap_or(Decimal::new(13, 2));
            let tax_amount = amount * tax_percent / Decimal::new(100, 0);
            let discount_percent = item_req.discount_percent.unwrap_or(Decimal::ZERO);
            let discount_amount = amount * discount_percent / Decimal::new(100, 0);

            purchase_order_item::ActiveModel {
                id: Set(0),
                order_id: Set(order.id),
                product_id: Set(item_req.material_id), // 使用 material_id 作为 product_id
                quantity: Set(quantity_ordered),
                quantity_alt: Set(quantity_alt_ordered),
                unit_price: Set(unit_price),
                unit_price_foreign: Set(unit_price), // 暂时使用相同值
                discount_percent: Set(discount_percent),
                tax_percent: Set(tax_percent),
                subtotal: Set(amount),
                tax_amount: Set(tax_amount),
                discount_amount: Set(discount_amount),
                total_amount: Set(amount + tax_amount - discount_amount),
                received_quantity: Set(Decimal::ZERO),
                received_quantity_alt: Set(Decimal::ZERO),
                notes: Set(item_req.notes),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            }
            .insert(&txn)
            .await?;
        }

        // 4. 更新订单总金额和数量
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        let order = order_active.update(&txn).await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(order)
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
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != "DRAFT" && order.order_status != "REJECTED" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许修改，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
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

        let order = order_active.update(&*self.db).await?;

        Ok(order)
    }

    /// 删除采购订单（仅草稿状态）
    pub async fn delete_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许删除，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能删除自己创建的订单".to_string(),
            ));
        }

        // 4. 删除订单（级联删除明细）
        purchase_order::Entity::delete_by_id(order_id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 提交采购订单
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != "DRAFT" && order.order_status != "REJECTED" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许提交，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能提交自己创建的订单".to_string(),
            ));
        }

        // 4. 检查是否有明细
        let item_count = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .count(&*self.db)
            .await?;

        if item_count == 0 {
            return Err(AppError::BusinessError("订单至少需要一行明细".to_string()));
        }

        // 5. 更新状态
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set("SUBMITTED".to_string());
        order_active.updated_by = Set(Some(user_id));

        let order = order_active.update(&*self.db).await?;

        Ok(order)
    }

    /// 审批采购订单
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != "SUBMITTED" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许审批，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set("APPROVED".to_string());
        order_active.approved_by = Set(Some(user_id));
        order_active.approved_at = Set(Some(now));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = order_active.update(&*self.db).await?;

        Ok(order)
    }

    /// 拒绝采购订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != "SUBMITTED" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许拒绝，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set("REJECTED".to_string());
        order_active.rejected_reason = Set(Some(reason));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = order_active.update(&*self.db).await?;

        Ok(order)
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
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态（已完成或部分入库的订单才能关闭）
        if !["COMPLETED", "PARTIAL_RECEIVED"].contains(&order.order_status.as_str()) {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许关闭，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set("CLOSED".to_string());
        order_active.updated_by = Set(Some(user_id));

        let order = order_active.update(&*self.db).await?;

        Ok(order)
    }

    /// 添加订单明细
    pub async fn add_order_item(
        &self,
        order_id: i32,
        req: CreateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许添加明细，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能为自己创建的订单添加明细".to_string(),
            ));
        }

        // 4. 创建明细
        let quantity_ordered = req.quantity_ordered;
        let unit_price = req.unit_price;
        let amount = quantity_ordered * unit_price;
        let tax_percent = req.tax_rate.unwrap_or(Decimal::new(13, 2));
        let tax_amount = amount * tax_percent / Decimal::new(100, 0);
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let discount_amount = amount * discount_percent / Decimal::new(100, 0);
        let quantity_alt_ordered = req.quantity_alt_ordered.unwrap_or(Decimal::ZERO);

        let item = purchase_order_item::ActiveModel {
            id: Set(0),
            order_id: Set(order_id),
            product_id: Set(req.material_id), // 使用 material_id 作为 product_id
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
            notes: Set(req.notes),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
        .insert(&*self.db)
        .await?;

        // 5. 更新订单总金额
        self.calculate_order_total(order_id).await?;

        Ok(item)
    }

    /// 更新订单明细
    pub async fn update_order_item(
        &self,
        item_id: i32,
        req: UpdateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购订单 {}",
                item.order_id
            )))?;

        // 3. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许修改明细，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能修改自己创建的订单明细".to_string(),
            ));
        }

        // 5. 更新明细
        let mut item_active: purchase_order_item::ActiveModel = item.into();

        if let Some(material_id) = req.material_id {
            item_active.product_id = Set(material_id);
        }
        if let Some(unit_price) = req.unit_price {
            item_active.unit_price = Set(unit_price);
        }
        if let Some(quantity) = req.quantity_ordered {
            item_active.quantity = Set(quantity);
        }
        if let Some(tax_rate) = req.tax_rate {
            item_active.tax_percent = Set(tax_rate);
        }
        if let Some(notes) = req.notes {
            item_active.notes = Set(Some(notes));
        }

        let item = item_active.update(&*self.db).await?;

        // 6. 更新订单总金额
        self.calculate_order_total(order.id).await?;

        Ok(item)
    }

    /// 删除订单明细
    pub async fn delete_order_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购订单 {}",
                item.order_id
            )))?;

        // 3. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "订单状态不允许删除明细，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 检查权限
        if order.created_by != user_id {
            return Err(AppError::PermissionDenied(
                "只能删除自己创建的订单明细".to_string(),
            ));
        }

        // 5. 删除明细
        purchase_order_item::Entity::delete_by_id(item_id)
            .exec(&*self.db)
            .await?;

        // 6. 更新订单总金额
        self.calculate_order_total(order.id).await?;

        Ok(())
    }

    /// 计算订单总金额
    pub async fn calculate_order_total(&self, order_id: i32) -> Result<(), AppError> {
        // 1. 查询所有明细
        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await?;

        // 2. 计算总和
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        for item in items {
            total_amount += item.total_amount;
            total_quantity += item.quantity;
            total_quantity_alt += item.quantity_alt;
        }

        // 3. 更新订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        order_active.update(&*self.db).await?;

        Ok(())
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
            .ok_or(AppError::ResourceNotFound(format!("采购订单 {}", order_id)))?;

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
}

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购订单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 订单日期
    pub order_date: NaiveDate,

    /// 预计交货日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 部门 ID
    pub department_id: i32,

    /// 币种
    pub currency: Option<String>,

    /// 汇率
    pub exchange_rate: Option<Decimal>,

    /// 付款条件
    pub payment_terms: Option<String>,

    /// 运输条款
    pub shipping_terms: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 订单明细
    #[validate(length(min = 1, message = "订单至少需要一行明细"))]
    pub items: Vec<CreateOrderItemRequest>,
}

/// 更新采购订单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseOrderRequest {
    pub supplier_id: Option<i32>,
    pub order_date: Option<NaiveDate>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

/// 创建订单明细请求
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct CreateOrderItemRequest {
    /// 行号
    pub line_no: i32,

    /// 物料 ID
    pub material_id: i32,

    /// 物料编码
    pub material_code: String,

    /// 物料名称
    pub material_name: String,

    /// 规格型号
    pub specification: Option<String>,

    /// 批次号
    pub batch_no: Option<String>,

    /// 色号
    pub color_code: Option<String>,

    /// 缸号
    pub lot_no: Option<String>,

    /// 等级
    pub grade: Option<String>,

    /// 克重
    pub gram_weight: Option<Decimal>,

    /// 幅宽
    pub width: Option<Decimal>,

    /// 单价
    pub unit_price: Decimal,

    /// 币种
    pub currency: Option<String>,

    /// 订购数量（主单位）
    pub quantity_ordered: Decimal,

    /// 主单位
    pub unit_master: String,

    /// 辅助单位
    pub unit_alt: Option<String>,

    /// 换算系数
    pub conversion_factor: Option<Decimal>,

    /// 订购数量（辅助单位）
    pub quantity_alt_ordered: Option<Decimal>,

    /// 税率
    pub tax_rate: Option<Decimal>,

    /// 折扣百分比
    pub discount_percent: Option<Decimal>,

    /// 交货日期
    pub delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新订单明细请求
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct UpdateOrderItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub quantity_ordered: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub delivery_date: Option<NaiveDate>,
    pub notes: Option<String>,
}
