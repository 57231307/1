use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait, QuerySelect, QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::models::dto::PageRequest;
use crate::models::inventory_reservation::{self, Entity as InventoryReservationEntity};
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::sales_order::{self, Entity as SalesOrderEntity};
use crate::models::sales_order_item::{self, Entity as SalesOrderItemEntity};
use crate::utils::PaginatedResponse;
use serde::{Deserialize, Serialize};
use sea_orm::{FromQueryResult, JoinType, RelationTrait};

/// 销售订单详情响应
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderDetail {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub order_date: chrono::DateTime<chrono::Utc>,
    pub required_date: chrono::DateTime<chrono::Utc>,
    pub ship_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub subtotal: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub discount_amount: rust_decimal::Decimal,
    pub shipping_cost: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub paid_amount: rust_decimal::Decimal,
    pub balance_amount: rust_decimal::Decimal,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[sea_orm(skip)]
    pub items: Vec<SalesOrderItemDetail>,
}

/// 销售订单明细项详情
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderItemDetail {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub discount_percent: rust_decimal::Decimal,
    pub tax_percent: rust_decimal::Decimal,
    pub subtotal: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub discount_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub shipped_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub color_no: String,
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub grade_required: Option<String>,
    pub quantity_meters: rust_decimal::Decimal,
    pub quantity_kg: rust_decimal::Decimal,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub color_extra_cost: rust_decimal::Decimal,
    pub grade_price_diff: rust_decimal::Decimal,
    pub final_price: Option<rust_decimal::Decimal>,
    pub shipped_quantity_meters: rust_decimal::Decimal,
    pub shipped_quantity_kg: rust_decimal::Decimal,
}

/// 创建销售订单请求
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateSalesOrderRequest {
    pub customer_id: i32,
    pub required_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<SalesOrderItemRequest>,
    // 面料行业特有字段
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SalesOrderItemRequest {
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub discount_percent: Option<rust_decimal::Decimal>,
    pub tax_percent: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    pub color_no: Option<String>,
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub grade_required: Option<String>,
    pub quantity_meters: Option<rust_decimal::Decimal>,
    pub quantity_kg: Option<rust_decimal::Decimal>,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub color_extra_cost: Option<rust_decimal::Decimal>,
    pub grade_price_diff: Option<rust_decimal::Decimal>,
    pub final_price: Option<rust_decimal::Decimal>,
}

/// 更新销售订单请求
#[derive(Debug, Deserialize)]
pub struct UpdateSalesOrderRequest {
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<SalesOrderItemRequest>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShipOrderItemRequest {
    pub order_item_id: i32,
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub warehouse_id: i32,
    pub batch_no: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShipOrderRequest {
    pub items: Vec<ShipOrderItemRequest>,
}

/// 销售订单服务
pub struct SalesService {
    db: Arc<DatabaseConnection>,
}

impl SalesService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取销售订单列表
    pub async fn list_orders(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_id: Option<i32>,
        order_no: Option<String>,
    ) -> Result<PaginatedResponse<SalesOrderDetail>, sea_orm::DbErr> {
        let mut query = SalesOrderEntity::find()
            .column_as(crate::models::customer::Column::CustomerName, "customer_name")
            .join(JoinType::LeftJoin, sales_order::Relation::Customer.def());

        // 应用过滤条件
        if let Some(s) = status {
            query = query.filter(sales_order::Column::Status.eq(s));
        }
        if let Some(cid) = customer_id {
            query = query.filter(sales_order::Column::CustomerId.eq(cid));
        }
        if let Some(no) = order_no {
            query = query.filter(sales_order::Column::OrderNo.contains(&no));
        }

        // 分页
        let paginator = query
            .order_by(sales_order::Column::CreatedAt, Order::Desc)
            .into_model::<SalesOrderDetail>()
            .paginate(&*self.db, page_req.page_size);

        let total = paginator.num_items().await?;
        let mut order_details: Vec<SalesOrderDetail> =
            paginator.fetch_page(page_req.page - 1).await?;

        // 列表接口不返回明细项，确保 items 初始化
        for order in &mut order_details {
            order.items = vec![];
        }

        Ok(PaginatedResponse::new(
            order_details,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取销售订单详情（包含明细项）
    pub async fn get_order_detail(
        &self,
        order_id: i32,
    ) -> Result<SalesOrderDetail, sea_orm::DbErr> {
        // 获取订单主表数据
        let mut order = SalesOrderEntity::find_by_id(order_id)
            .column_as(crate::models::customer::Column::CustomerName, "customer_name")
            .join(JoinType::LeftJoin, sales_order::Relation::Customer.def())
            .into_model::<SalesOrderDetail>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("销售订单 {} 未找到", order_id))
            })?;

        // 获取订单明细项
        use crate::models::product;
        let items = SalesOrderItemEntity::find()
            .column_as(product::Column::Code, "product_code")
            .column_as(product::Column::Name, "product_name")
            .join(JoinType::LeftJoin, sales_order_item::Relation::Product.def())
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .order_by(sales_order_item::Column::Id, Order::Asc)
            .into_model::<SalesOrderItemDetail>()
            .all(&*self.db)
            .await?;

        order.items = items;
        Ok(order)
    }

    /// 创建销售订单
    pub async fn create_order(
        &self,
        request: CreateSalesOrderRequest,
    ) -> Result<SalesOrderDetail, sea_orm::DbErr> {
        // 开启事务
        let txn = (*self.db).begin().await?;

        // 客户信用风控拦截
        let customer = crate::models::customer::Entity::find_by_id(request.customer_id)
            .one(&txn)
            .await?
            .ok_or(sea_orm::DbErr::Custom(format!("客户 {} 不存在", request.customer_id)))?;
            
        let credit_limit = customer.credit_limit;
        
        // 计算当前未付应收账款总额 (AR invoices that are not 'paid')
        use sea_orm::{QuerySelect, QueryFilter, ColumnTrait};
        let unpaid_invoices = crate::models::finance_invoice::Entity::find()
            .filter(crate::models::finance_invoice::Column::CustomerId.eq(request.customer_id))
            .filter(crate::models::finance_invoice::Column::InvoiceType.eq("AR"))
            .filter(crate::models::finance_invoice::Column::Status.ne("paid"))
            .all(&txn)
            .await?;
            
        let mut total_unpaid = rust_decimal::Decimal::new(0, 0);
        for inv in unpaid_invoices {
            total_unpaid += inv.total_amount;
        }
        
        // 计算本单金额
        let mut order_amount = rust_decimal::Decimal::new(0, 0);
        for item in &request.items {
            let qty = item.quantity;
            let price = item.unit_price;
            let discount = item.discount_percent.unwrap_or(rust_decimal::Decimal::new(0, 0));
            let tax = item.tax_percent.unwrap_or(rust_decimal::Decimal::new(0, 0));
            
            let mut subtotal = qty * price;
            if discount > rust_decimal::Decimal::new(0, 0) {
                let disc_amt = subtotal * discount / rust_decimal::Decimal::new(100, 0);
                subtotal -= disc_amt;
            }
            if tax > rust_decimal::Decimal::new(0, 0) {
                let tax_amt = subtotal * tax / rust_decimal::Decimal::new(100, 0);
                subtotal += tax_amt;
            }
            order_amount += subtotal;
        }
        
        // 判断是否超额
        if credit_limit > rust_decimal::Decimal::new(0, 0) && (total_unpaid + order_amount) > credit_limit {
            txn.rollback().await?;
            return Err(sea_orm::DbErr::Custom(format!(
                "信用风控拦截：客户当前未付账款 {} + 本单金额 {} = {}，超出了信用额度 {}",
                total_unpaid, order_amount, total_unpaid + order_amount, credit_limit
            )));
        }

        // 生成订单号并检查唯一性
        let order_no = self.generate_order_no().await?;

        // 再次检查订单号是否已存在（防止并发冲突）
        let existing_order = SalesOrderEntity::find()
            .filter(sales_order::Column::OrderNo.eq(&order_no))
            .one(&txn)
            .await?;

        if existing_order.is_some() {
            txn.rollback().await?;
            return Err(sea_orm::DbErr::Custom("订单号已存在，请重试".to_string()));
        }

        // 创建订单主表
        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: sea_orm::ActiveValue::Set(order_no),
            customer_id: sea_orm::ActiveValue::Set(request.customer_id),
            order_date: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            required_date: sea_orm::ActiveValue::Set(request.required_date),
            ship_date: sea_orm::ActiveValue::NotSet,
            status: sea_orm::ActiveValue::Set(request.status),
            subtotal: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            tax_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            discount_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            shipping_cost: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            total_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            paid_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            balance_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            shipping_address: sea_orm::ActiveValue::Set(request.shipping_address),
            billing_address: sea_orm::ActiveValue::Set(request.billing_address),
            notes: sea_orm::ActiveValue::Set(request.notes),
            created_by: sea_orm::ActiveValue::NotSet,
            approved_by: sea_orm::ActiveValue::NotSet,
            approved_at: sea_orm::ActiveValue::NotSet,
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };

        let order_entity = order.insert(&txn).await?;

        // 检查库存是否充足
        self.check_inventory(&request.items, &txn).await?;

        // 创建订单明细项并计算金额
        let mut subtotal = rust_decimal::Decimal::ZERO;
        let mut tax_amount = rust_decimal::Decimal::ZERO;
        let mut discount_amount = rust_decimal::Decimal::ZERO;
        let mut total_amount = rust_decimal::Decimal::ZERO;

        for item_req in request.items {
            let discount_pct = item_req
                .discount_percent
                .unwrap_or(rust_decimal::Decimal::ZERO);
            let tax_pct = item_req.tax_percent.unwrap_or(rust_decimal::Decimal::ZERO);

            // 计算明细项金额
            let item_subtotal = item_req.quantity * item_req.unit_price;
            let item_discount =
                item_subtotal * (discount_pct / rust_decimal::Decimal::new(100, 0));
            let item_after_discount = item_subtotal - item_discount;
            let item_tax = item_after_discount * (tax_pct / rust_decimal::Decimal::new(100, 0));
            let item_total = item_after_discount + item_tax;

            // 累加订单总额
            subtotal += &item_subtotal;
            discount_amount += &item_discount;
            tax_amount += &item_tax;
            total_amount += &item_total;

            // 创建明细项
            let item = sales_order_item::ActiveModel {
                id: Default::default(),
                order_id: sea_orm::ActiveValue::Set(order_entity.id),
                product_id: sea_orm::ActiveValue::Set(item_req.product_id),
                quantity: sea_orm::ActiveValue::Set(item_req.quantity),
                unit_price: sea_orm::ActiveValue::Set(item_req.unit_price),
                discount_percent: sea_orm::ActiveValue::Set(discount_pct),
                tax_percent: sea_orm::ActiveValue::Set(tax_pct),
                subtotal: sea_orm::ActiveValue::Set(item_subtotal),
                tax_amount: sea_orm::ActiveValue::Set(item_tax),
                discount_amount: sea_orm::ActiveValue::Set(item_discount),
                total_amount: sea_orm::ActiveValue::Set(item_total),
                shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                notes: sea_orm::ActiveValue::Set(item_req.notes),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                color_no: sea_orm::ActiveValue::Set(item_req.color_no.unwrap_or_default()),
                color_name: sea_orm::ActiveValue::Set(item_req.color_name),
                pantone_code: sea_orm::ActiveValue::Set(item_req.pantone_code),
                grade_required: sea_orm::ActiveValue::Set(item_req.grade_required),
                quantity_meters: sea_orm::ActiveValue::Set(
                    item_req
                        .quantity_meters
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                ),
                quantity_kg: sea_orm::ActiveValue::Set(
                    item_req.quantity_kg.unwrap_or(rust_decimal::Decimal::ZERO),
                ),
                gram_weight: sea_orm::ActiveValue::Set(item_req.gram_weight),
                width: sea_orm::ActiveValue::Set(item_req.width),
                batch_requirement: sea_orm::ActiveValue::Set(item_req.batch_requirement),
                dye_lot_requirement: sea_orm::ActiveValue::Set(item_req.dye_lot_requirement),
                base_price: sea_orm::ActiveValue::Set(item_req.base_price),
                color_extra_cost: sea_orm::ActiveValue::Set(
                    item_req
                        .color_extra_cost
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                ),
                grade_price_diff: sea_orm::ActiveValue::Set(
                    item_req
                        .grade_price_diff
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                ),
                final_price: sea_orm::ActiveValue::Set(item_req.final_price),
                shipped_quantity_meters: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                shipped_quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                paper_tube_weight: sea_orm::ActiveValue::Set(item_req.paper_tube_weight),
                is_net_weight: sea_orm::ActiveValue::Set(item_req.is_net_weight),
            };

            item.insert(&txn).await?;
        }

        // 更新订单总金额
        let order_id = order_entity.id;
        let mut order_update: sales_order::ActiveModel = order_entity.into();
        order_update.subtotal = sea_orm::ActiveValue::Set(subtotal);
        order_update.tax_amount = sea_orm::ActiveValue::Set(tax_amount);
        order_update.discount_amount = sea_orm::ActiveValue::Set(discount_amount);
        order_update.total_amount = sea_orm::ActiveValue::Set(total_amount);
        order_update.balance_amount = sea_orm::ActiveValue::Set(total_amount);
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        order_update.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(order_id).await
    }

    /// 更新销售订单
    pub async fn update_order(
        &self,
        order_id: i32,
        request: UpdateSalesOrderRequest,
    ) -> Result<SalesOrderDetail, sea_orm::DbErr> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("销售订单 {} 未找到", order_id))
            })?;

        // 检查订单状态，已发货或已完成的订单不允许修改
        if order.status == "shipped" || order.status == "completed" {
            return Err(sea_orm::DbErr::Custom(format!(
                "订单状态为{}，不允许修改",
                order.status
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新订单主表
        let mut order_update: sales_order::ActiveModel = order.into();
        if let Some(required_date) = request.required_date {
            order_update.required_date = sea_orm::ActiveValue::Set(required_date);
        }
        if let Some(status) = request.status {
            order_update.status = sea_orm::ActiveValue::Set(status);
        }
        if let Some(shipping_address) = request.shipping_address {
            order_update.shipping_address = sea_orm::ActiveValue::Set(Some(shipping_address));
        }
        if let Some(billing_address) = request.billing_address {
            order_update.billing_address = sea_orm::ActiveValue::Set(Some(billing_address));
        }
        if let Some(notes) = request.notes {
            order_update.notes = sea_orm::ActiveValue::Set(Some(notes));
        }
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        order_update.update(&txn).await?;

        // 如果需要更新明细项
        if let Some(items) = request.items {
            // 删除原有明细项
            SalesOrderItemEntity::delete_many()
                .filter(sales_order_item::Column::OrderId.eq(order_id))
                .exec(&txn)
                .await?;

            // 重新计算金额并创建新明细项
            let mut subtotal = rust_decimal::Decimal::ZERO;
            let mut tax_amount = rust_decimal::Decimal::ZERO;
            let mut discount_amount = rust_decimal::Decimal::ZERO;
            let mut total_amount = rust_decimal::Decimal::ZERO;

            for item_req in items {
                let discount_pct = item_req
                    .discount_percent
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let tax_pct = item_req.tax_percent.unwrap_or(rust_decimal::Decimal::ZERO);

                let item_subtotal = item_req.quantity * item_req.unit_price;
                let item_discount =
                    item_subtotal * (discount_pct / rust_decimal::Decimal::new(100, 0));
                let item_after_discount = item_subtotal - item_discount;
                let item_tax =
                    item_after_discount * (tax_pct / rust_decimal::Decimal::new(100, 0));
                let item_total = item_after_discount + item_tax;

                subtotal += &item_subtotal;
                discount_amount += &item_discount;
                tax_amount += &item_tax;
                total_amount += &item_total;

                let item = sales_order_item::ActiveModel {
                    id: Default::default(),
                    order_id: sea_orm::ActiveValue::Set(order_id),
                    product_id: sea_orm::ActiveValue::Set(item_req.product_id),
                    quantity: sea_orm::ActiveValue::Set(item_req.quantity),
                    unit_price: sea_orm::ActiveValue::Set(item_req.unit_price),
                    discount_percent: sea_orm::ActiveValue::Set(discount_pct),
                    tax_percent: sea_orm::ActiveValue::Set(tax_pct),
                    subtotal: sea_orm::ActiveValue::Set(item_subtotal),
                    tax_amount: sea_orm::ActiveValue::Set(item_tax),
                    discount_amount: sea_orm::ActiveValue::Set(item_discount),
                    total_amount: sea_orm::ActiveValue::Set(item_total),
                    shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    notes: sea_orm::ActiveValue::Set(item_req.notes),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    color_no: sea_orm::ActiveValue::Set(item_req.color_no.unwrap_or_default()),
                    color_name: sea_orm::ActiveValue::Set(item_req.color_name),
                    pantone_code: sea_orm::ActiveValue::Set(item_req.pantone_code),
                    grade_required: sea_orm::ActiveValue::Set(item_req.grade_required),
                    quantity_meters: sea_orm::ActiveValue::Set(
                        item_req
                            .quantity_meters
                            .unwrap_or(rust_decimal::Decimal::ZERO),
                    ),
                    quantity_kg: sea_orm::ActiveValue::Set(
                        item_req.quantity_kg.unwrap_or(rust_decimal::Decimal::ZERO),
                    ),
                    gram_weight: sea_orm::ActiveValue::Set(item_req.gram_weight),
                    width: sea_orm::ActiveValue::Set(item_req.width),
                    batch_requirement: sea_orm::ActiveValue::Set(item_req.batch_requirement),
                    dye_lot_requirement: sea_orm::ActiveValue::Set(item_req.dye_lot_requirement),
                    base_price: sea_orm::ActiveValue::Set(item_req.base_price),
                    color_extra_cost: sea_orm::ActiveValue::Set(
                        item_req
                            .color_extra_cost
                            .unwrap_or(rust_decimal::Decimal::ZERO),
                    ),
                    grade_price_diff: sea_orm::ActiveValue::Set(
                        item_req
                            .grade_price_diff
                            .unwrap_or(rust_decimal::Decimal::ZERO),
                    ),
                    final_price: sea_orm::ActiveValue::Set(item_req.final_price),
                    shipped_quantity_meters: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                        shipped_quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                        paper_tube_weight: sea_orm::ActiveValue::Set(item_req.paper_tube_weight),
                        is_net_weight: sea_orm::ActiveValue::Set(item_req.is_net_weight),
                    };

                item.insert(&txn).await?;
            }

            // 更新订单总金额
            let order_entity = SalesOrderEntity::find_by_id(order_id)
                .one(&txn)
                .await?
                .ok_or_else(|| sea_orm::DbErr::Custom("销售订单不存在".to_string()))?;
            let mut order_update: sales_order::ActiveModel = order_entity.into();
            order_update.subtotal = sea_orm::ActiveValue::Set(subtotal);
            order_update.tax_amount = sea_orm::ActiveValue::Set(tax_amount);
            order_update.discount_amount = sea_orm::ActiveValue::Set(discount_amount);
            order_update.total_amount = sea_orm::ActiveValue::Set(total_amount);
            order_update.balance_amount = sea_orm::ActiveValue::Set(total_amount);
            order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            order_update.update(&txn).await?;
        }

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(order_id).await
    }

    /// 删除销售订单
    pub async fn delete_order(&self, order_id: i32) -> Result<(), sea_orm::DbErr> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("销售订单 {} 未找到", order_id))
            })?;

        // 检查订单状态，已发货或已完成的订单不允许删除
        if order.status == "shipped" || order.status == "completed" {
            return Err(sea_orm::DbErr::Custom(format!(
                "订单状态为{}，不允许删除",
                order.status
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 删除订单明细项
        SalesOrderItemEntity::delete_many()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .exec(&txn)
            .await?;

        // 删除订单主表
        SalesOrderEntity::delete_by_id(order_id).exec(&txn).await?;

        // 提交事务
        txn.commit().await?;

        Ok(())
    }

    /// 生成订单号
    async fn generate_order_no(&self) -> Result<String, sea_orm::DbErr> {
        let now = chrono::Utc::now();
        let date_str = now.format("%Y%m%d").to_string();

        // 获取当天最大订单号
        let max_order = SalesOrderEntity::find()
            .filter(sales_order::Column::OrderNo.like(format!("SO{}%", date_str)))
            .order_by(sales_order::Column::OrderNo, Order::Desc)
            .one(&*self.db)
            .await?;

        let seq = match max_order {
            Some(order) => {
                // 提取序号部分并加 1
                let seq_str = order
                    .order_no
                    .trim_start_matches(&format!("SO{}", date_str));
                seq_str.parse::<u32>().unwrap_or(0) + 1
            }
            None => 1,
        };

        Ok(format!("SO{}{:04}", date_str, seq))
    }

    /// 检查库存是否充足
    async fn check_inventory(
        &self,
        items: &[SalesOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), sea_orm::DbErr> {
        for item in items {
            // 查询产品库存（假设使用默认仓库，实际应从产品配置获取）
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(txn)
                .await?;

            match stock {
                Some(s) if s.quantity_available >= item.quantity => {
                    // 库存充足，继续检查下一个产品
                    continue;
                }
                Some(s) => {
                    return Err(sea_orm::DbErr::Custom(format!(
                        "产品 {} 库存不足，当前库存：{}，需要：{}",
                        item.product_id, s.quantity_available, item.quantity
                    )));
                }
                None => {
                    return Err(sea_orm::DbErr::Custom(format!(
                        "产品 {} 没有库存记录",
                        item.product_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// 锁定库存（创建预留记录）
    async fn lock_inventory(
        &self,
        order_id: i32,
        items: &[SalesOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), sea_orm::DbErr> {
        // 为每个订单明细项创建库存预留记录
        for item in items {
            // 查询产品库存（假设使用默认仓库）
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(txn)
                .await?;

            if let Some(s) = stock {
                if s.quantity_available < item.quantity {
                    return Err(sea_orm::DbErr::Custom(format!(
                        "产品 {} 库存不足，无法锁定",
                        item.product_id
                    )));
                }

                // 创建库存预留记录
                let reservation = inventory_reservation::ActiveModel {
                    id: Default::default(),
                    order_id: sea_orm::ActiveValue::Set(order_id),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(s.warehouse_id),
                    quantity: sea_orm::ActiveValue::Set(item.quantity),
                    status: sea_orm::ActiveValue::Set("pending".to_string()),
                    reserved_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    released_at: sea_orm::ActiveValue::NotSet,
                    notes: sea_orm::ActiveValue::NotSet,
                    created_by: sea_orm::ActiveValue::NotSet,
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                reservation.insert(txn).await?;
            } else {
                return Err(sea_orm::DbErr::Custom(format!(
                    "产品 {} 没有库存记录，无法锁定",
                    item.product_id
                )));
            }
        }
        Ok(())
    }

    /// 扣减库存
    async fn reduce_inventory(
        &self,
        order_id: i32,
        ship_items: &Vec<ShipOrderItemRequest>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), sea_orm::DbErr> {
        for item in ship_items {
            // 查询库存记录
            use sea_orm::QuerySelect;
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .filter(inventory_stock::Column::WarehouseId.eq(item.warehouse_id))
                .filter(inventory_stock::Column::BatchNo.eq(&item.batch_no))
                .lock_exclusive()
                .one(txn)
                .await?;

            if let Some(s) = stock {
                // 检查库存是否充足
                if s.quantity_on_hand < item.quantity {
                    return Err(sea_orm::DbErr::Custom(format!(
                        "产品 {} 在仓库 {} (批次 {}) 库存不足，当前可用 {}，需要 {}",
                        item.product_id, item.warehouse_id, item.batch_no, s.quantity_on_hand, item.quantity
                    )));
                }

                // 扣减库存
                let new_quantity_on_hand = s.quantity_on_hand - item.quantity;
                let new_quantity_available = s.quantity_available - item.quantity;
                let stock_update = inventory_stock::ActiveModel {
                    id: sea_orm::ActiveValue::Unchanged(s.id),
                    quantity_on_hand: sea_orm::ActiveValue::Set(new_quantity_on_hand),
                    quantity_available: sea_orm::ActiveValue::Set(new_quantity_available),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    ..Default::default()
                };
                stock_update.update(txn).await?;

                // 查找对应的预留记录并标记为已使用
                let reservation = InventoryReservationEntity::find()
                    .filter(inventory_reservation::Column::OrderId.eq(order_id))
                    .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
                    .filter(inventory_reservation::Column::WarehouseId.eq(item.warehouse_id))
                    .filter(inventory_reservation::Column::Status.eq("pending"))
                    .one(txn)
                    .await?;

                if let Some(res) = reservation {
                    let mut res_update: inventory_reservation::ActiveModel = res.into();
                    res_update.status = sea_orm::ActiveValue::Set("used".to_string());
                    res_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                    res_update.update(txn).await?;
                }
            } else {
                return Err(sea_orm::DbErr::Custom(format!(
                    "产品 {} 在仓库 {} (批次 {}) 没有库存记录，无法扣减",
                    item.product_id, item.warehouse_id, item.batch_no
                )));
            }
            
            // 更新订单明细的已发货数量
            let order_item = SalesOrderItemEntity::find_by_id(item.order_item_id)
                .one(txn)
                .await?;
            if let Some(oi) = order_item {
                let mut oi_update: sales_order_item::ActiveModel = oi.into();
                let current_shipped = match oi_update.shipped_quantity.clone() {
                    sea_orm::ActiveValue::Set(v) => v,
                    _ => rust_decimal::Decimal::new(0, 0),
                };
                oi_update.shipped_quantity = sea_orm::ActiveValue::Set(current_shipped + item.quantity);
                oi_update.update(txn).await?;
            }
        }
        Ok(())
    }

    /// 审核销售订单
    pub async fn approve_order(&self, order_id: i32) -> Result<SalesOrderDetail, sea_orm::DbErr> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("销售订单 {} 未找到", order_id))
            })?;

        // 检查订单状态，只有草稿状态可以审核
        if order.status != "draft" {
            return Err(sea_orm::DbErr::Custom(format!(
                "订单状态为{}，只有草稿状态的订单可以审核",
                order.status
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 锁定库存
        let order_detail = self.get_order_detail(order_id).await?;
        let items: Vec<SalesOrderItemRequest> = order_detail
            .items
            .iter()
            .map(|item| SalesOrderItemRequest {
                product_id: item.product_id,
                quantity: item.quantity,
                unit_price: item.unit_price,
                discount_percent: Some(item.discount_percent),
                tax_percent: Some(item.tax_percent),
                notes: item.notes.clone(),
                color_no: Some(item.color_no.clone()),
                color_name: item.color_name.clone(),
                pantone_code: item.pantone_code.clone(),
                grade_required: item.grade_required.clone(),
                quantity_meters: Some(item.quantity_meters),
                quantity_kg: Some(item.quantity_kg),
                gram_weight: item.gram_weight,
                width: item.width,
                batch_requirement: item.batch_requirement.clone(),
                dye_lot_requirement: item.dye_lot_requirement.clone(),
                base_price: item.base_price,
                color_extra_cost: Some(item.color_extra_cost),
                grade_price_diff: Some(item.grade_price_diff),
                final_price: item.final_price,
                paper_tube_weight: item.paper_tube_weight,
                is_net_weight: item.is_net_weight,
            })
            .collect();

        self.lock_inventory(order_id, &items, &txn).await?;

        // 更新订单状态为已审核
        let updated_order = sales_order::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(order.id),
            status: sea_orm::ActiveValue::Set("approved".to_string()),
            approved_by: sea_orm::ActiveValue::Set(order.created_by), // 使用创建人作为审核人
            approved_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now())),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let approved_order = updated_order.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(approved_order.id).await
    }

    /// 发货处理
    pub async fn ship_order(&self, order_id: i32, req: ShipOrderRequest) -> Result<SalesOrderDetail, sea_orm::DbErr> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("销售订单 {} 未找到", order_id))
            })?;

        // 检查订单状态，只有已审核状态可以发货
        if order.status != "approved" {
            return Err(sea_orm::DbErr::Custom(format!(
                "订单状态为{}，只有已审核状态的订单可以发货",
                order.status
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 扣减库存
        self.reduce_inventory(order_id, &req.items, &txn).await?;

        // 更新订单状态为已发货
        let updated_order = sales_order::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(order.id),
            status: sea_orm::ActiveValue::Set("shipped".to_string()),
            ship_date: sea_orm::ActiveValue::Set(Some(chrono::Utc::now())),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let shipped_order = updated_order.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(shipped_order.id).await
    }

    /// 完成订单
    pub async fn complete_order(&self, order_id: i32) -> Result<SalesOrderDetail, sea_orm::DbErr> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("销售订单 {} 未找到", order_id))
            })?;

        // 检查订单状态，只有已发货状态可以完成
        if order.status != "shipped" {
            return Err(sea_orm::DbErr::Custom(format!(
                "订单状态为{}，只有已发货状态的订单可以完成",
                order.status
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新订单状态为已完成
        let updated_order = sales_order::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(order.id),
            status: sea_orm::ActiveValue::Set("completed".to_string()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let completed_order = updated_order.update(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 自动生成应收账款（AR）发票
        let finance_service = crate::services::finance_invoice_service::FinanceInvoiceService::new(self.db.clone());
        
        // 查询客户名称
        let customer = crate::models::customer::Entity::find_by_id(order.customer_id)
            .one(&*self.db)
            .await?;
            
        let customer_name = customer.map(|c| c.customer_name).unwrap_or_else(|| "未知客户".to_string());
        
        let invoice_req = crate::services::finance_invoice_service::CreateInvoiceRequest {
            invoice_no: format!("INV-{}", order.order_no),
            order_id: Some(order.id),
            customer_id: Some(order.customer_id),
            customer_name,
            invoice_type: "AR".to_string(),
            amount: order.total_amount,
            tax_amount: rust_decimal::Decimal::new(0, 0),
            total_amount: order.total_amount,
            status: Some("pending".to_string()),
            invoice_date: Some(chrono::Utc::now()),
            due_date: Some(chrono::Utc::now() + chrono::Duration::days(30)),
            payment_method: None,
            notes: Some(format!("系统自动生成应收账款：{}", order.order_no)),
        };
        
        if let Err(e) = finance_service.create_invoice(invoice_req).await {
            tracing::error!("自动生成应收账款失败 (订单 {}): {}", order.order_no, e);
        } else {
            tracing::info!("成功自动生成应收账款 (订单 {})", order.order_no);
        }

        // 返回订单详情
        self.get_order_detail(completed_order.id).await
    }
}
