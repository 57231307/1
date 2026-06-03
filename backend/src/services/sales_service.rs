#![allow(dead_code)]

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use std::sync::Arc;

use crate::models::ar_invoice::{self, Column as ArInvoiceColumn, Entity as ArInvoiceEntity};
use crate::models::customer;
use crate::models::dto::PageRequest;
use crate::models::inventory_reservation::{self, Entity as InventoryReservationEntity};
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::sales_order::{self, Entity as SalesOrderEntity};
use crate::models::sales_order_item::{self, Entity as SalesOrderItemEntity};
use crate::services::user_service::UserService;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::PaginatedResponse;
use chrono;
use sea_orm::{FromQueryResult, JoinType, RelationTrait};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 销售订单详情响应
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderDetail {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub opportunity_id: Option<i32>,
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
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSalesOrderRequest {
    #[validate(range(min = 1, message = "客户ID必须大于0"))]
    pub customer_id: i32,
    pub opportunity_id: Option<i32>,
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    #[validate(length(max = 50, message = "状态长度不能超过50个字符"))]
    pub status: Option<String>,
    #[validate(length(max = 500, message = "收货地址长度不能超过500个字符"))]
    pub shipping_address: Option<String>,
    #[validate(length(max = 500, message = "账单地址长度不能超过500个字符"))]
    pub billing_address: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
    #[validate(length(min = 1, message = "订单项不能为空"))]
    pub items: Vec<SalesOrderItemRequest>,
    // 面料行业特有字段
    #[validate(length(max = 100, message = "付款条件长度不能超过100个字符"))]
    pub payment_terms: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
    #[validate(length(max = 50, message = "批次号长度不能超过50个字符"))]
    pub batch_no: Option<String>,
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "染缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
    #[validate(length(max = 20, message = "等级长度不能超过20个字符"))]
    pub grade: Option<String>,
    #[validate(length(max = 200, message = "包装要求长度不能超过200个字符"))]
    pub packaging_requirement: Option<String>,
    #[validate(length(max = 200, message = "质量标准长度不能超过200个字符"))]
    pub quality_standard: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct SalesOrderItemRequest {
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub discount_percent: Option<rust_decimal::Decimal>,
    pub tax_percent: Option<rust_decimal::Decimal>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "颜色名称长度不能超过50个字符"))]
    pub color_name: Option<String>,
    #[validate(length(max = 50, message = "潘通色号长度不能超过50个字符"))]
    pub pantone_code: Option<String>,
    #[validate(length(max = 20, message = "要求等级长度不能超过20个字符"))]
    pub grade_required: Option<String>,
    pub quantity_meters: Option<rust_decimal::Decimal>,
    pub quantity_kg: Option<rust_decimal::Decimal>,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    #[validate(length(max = 100, message = "批次要求长度不能超过100个字符"))]
    pub batch_requirement: Option<String>,
    #[validate(length(max = 100, message = "染批要求长度不能超过100个字符"))]
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
    ) -> Result<PaginatedResponse<SalesOrderDetail>, AppError> {
        let mut query = SalesOrderEntity::find()
            .column_as(
                crate::models::customer::Column::CustomerName,
                "customer_name",
            )
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
    pub async fn get_order_detail(&self, order_id: i32) -> Result<SalesOrderDetail, AppError> {
        // 获取订单主表数据
        let mut order = SalesOrderEntity::find_by_id(order_id)
            .column_as(
                crate::models::customer::Column::CustomerName,
                "customer_name",
            )
            .join(JoinType::LeftJoin, sales_order::Relation::Customer.def())
            .into_model::<SalesOrderDetail>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 获取订单明细项
        use crate::models::product;
        let items = SalesOrderItemEntity::find()
            .column_as(product::Column::Code, "product_code")
            .column_as(product::Column::Name, "product_name")
            .join(
                JoinType::LeftJoin,
                sales_order_item::Relation::Product.def(),
            )
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
        user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        let txn = (*self.db).begin().await?;

        // 业务逻辑验证：检查客户是否存在
        let customer = crate::models::customer::Entity::find_by_id(request.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::business(format!("客户 {} 不存在", request.customer_id)))?;

        // 业务逻辑验证：日期合理性检查
        let required_date = request
            .required_date
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(30));
        if required_date < chrono::Utc::now() {
            tracing::error!("Transaction rolled back: 交付日期不能早于当前时间");
            if let Err(e) = txn.rollback().await {
                tracing::error!("事务回滚失败: {}", e);
            }
            return Err(AppError::business(
                "创建面料订单失败: 交付日期不能早于当前时间".to_string(),
            ));
        }

        let _credit_limit = customer.credit_limit;

        // 计算当前未付应收账款总额
        let _total_unpaid = {
            use crate::models::ar_invoice;
            use sea_orm::QueryFilter;

            let unpaid_result = ar_invoice::Entity::find()
                .filter(ar_invoice::Column::CustomerId.eq(request.customer_id))
                .filter(ar_invoice::Column::Status.ne("CANCELLED"))
                .filter(ar_invoice::Column::Status.ne("COMPLETED"))
                .all(&txn)
                .await;

            match unpaid_result {
                Ok(invoices) => invoices.iter().map(|i| i.invoice_amount).sum(),
                Err(_) => rust_decimal::Decimal::ZERO,
            }
        };

        // 计算本单金额
        let mut order_amount = rust_decimal::Decimal::new(0, 0);
        for item in &request.items {
            let qty = item.quantity;
            let price = item.unit_price;
            let discount = item
                .discount_percent
                .unwrap_or(rust_decimal::Decimal::new(0, 0));
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

        // 使用信用服务检查额度
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let order_amount_bigdecimal = {
            use bigdecimal::BigDecimal;
            BigDecimal::parse_bytes(order_amount.to_string().as_bytes(), 10)
                .unwrap_or_else(|| BigDecimal::from(0))
        };

        let credit_available = credit_service
            .check_credit_available(request.customer_id, order_amount_bigdecimal.clone())
            .await
            .map_err(|e| AppError::business(format!("信用检查失败: {}", e)))?;

        if !credit_available {
            tracing::error!("Transaction rolled back: 信用额度不足");
            txn.rollback().await?;
            return Err(AppError::business(format!(
                "信用额度不足：订单金额 {} 超出可用信用额度",
                order_amount
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
            tracing::error!("Transaction rolled back: 订单号 {} 已存在", order_no);
            txn.rollback().await?;
            return Err(AppError::business("订单号已存在，请重试"));
        }

        // 创建订单主表
        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: sea_orm::ActiveValue::Set(order_no),
            customer_id: sea_orm::ActiveValue::Set(request.customer_id),
            opportunity_id: sea_orm::ActiveValue::Set(request.opportunity_id),
            order_date: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            required_date: sea_orm::ActiveValue::Set(required_date),
            ship_date: sea_orm::ActiveValue::NotSet,
            status: sea_orm::ActiveValue::Set(
                request.status.unwrap_or_else(|| "draft".to_string()),
            ),
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

        // 检查库存并预留（使用inventory_reservation表）
        self.lock_inventory(order_entity.id, &request.items, user_id, &txn)
            .await?;

        // 创建订单明细项并计算金额
        let mut subtotal = rust_decimal::Decimal::ZERO;
        let mut tax_amount = rust_decimal::Decimal::ZERO;
        let mut discount_amount = rust_decimal::Decimal::ZERO;
        let mut total_amount = rust_decimal::Decimal::ZERO;

        // 验证产品是否存在（批量查询优化）
        {
            let mut product_ids = std::collections::HashSet::new();
            for item in &request.items {
                product_ids.insert(item.product_id);
            }
            if !product_ids.is_empty() {
                use crate::models::product;
                let existing_products = product::Entity::find()
                    .filter(
                        product::Column::Id.is_in(product_ids.iter().cloned().collect::<Vec<_>>()),
                    )
                    .all(&txn)
                    .await?;
                let existing_product_ids: std::collections::HashSet<i32> =
                    existing_products.into_iter().map(|p| p.id).collect();
                for product_id in product_ids {
                    if !existing_product_ids.contains(&product_id) {
                        tracing::error!("Transaction rolled back: 产品 ID {} 不存在", product_id);
                        if let Err(e) = txn.rollback().await {
                            tracing::error!("事务回滚失败: {}", e);
                        }
                        return Err(AppError::business(format!("产品 ID {} 不存在", product_id)));
                    }
                }
            }
        }

        for item_req in request.items {
            let discount_pct = item_req
                .discount_percent
                .unwrap_or(rust_decimal::Decimal::ZERO);
            let tax_pct = item_req.tax_percent.unwrap_or(rust_decimal::Decimal::ZERO);

            // 计算明细项金额
            let item_subtotal = item_req.quantity * item_req.unit_price;
            let item_discount = item_subtotal * (discount_pct / rust_decimal::Decimal::new(100, 0));
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
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        // 占用信用额度（必须在事务内，失败则回滚）
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let order_amount_bigdecimal = {
            use bigdecimal::BigDecimal;
            BigDecimal::parse_bytes(order_amount.to_string().as_bytes(), 10)
                .unwrap_or_else(|| BigDecimal::from(0))
        };
        credit_service
            .occupy_credit(
                request.customer_id,
                order_amount_bigdecimal.clone(),
                user_id,
            )
            .await
            .map_err(|e| {
                tracing::error!("信用额度占用失败，事务回滚: {}", e);
                AppError::business(format!("信用额度占用失败: {}", e))
            })?;
        tracing::info!(
            "客户 {} 信用额度占用成功，金额: {}",
            request.customer_id,
            order_amount
        );
        // 检查信用预警
        if let Ok(Some(warning)) = credit_service
            .check_credit_warning(request.customer_id)
            .await
        {
            tracing::warn!("信用预警: {}", warning);
        }

        // 订单回写商机：更新商机的 actual_amount 和 actual_close_date
        if let Some(opportunity_id) = request.opportunity_id {
            use crate::models::crm_opportunity;

            let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::business(format!("商机 {} 不存在", opportunity_id)))?;

            let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
            opp_active.actual_amount = sea_orm::ActiveValue::Set(Some(total_amount));
            opp_active.actual_close_date =
                sea_orm::ActiveValue::Set(Some(chrono::Utc::now().date_naive()));
            opp_active.opportunity_stage =
                sea_orm::ActiveValue::Set(Some("closed_won".to_string()));
            opp_active.opportunity_status = sea_orm::ActiveValue::Set(Some("won".to_string()));
            opp_active.updated_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));

            opp_active.update(&txn).await?;

            tracing::info!(
                "商机 {} 已关联订单并更新实际金额: {}",
                opportunity_id,
                total_amount
            );
        }

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
    ) -> Result<SalesOrderDetail, AppError> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 检查订单状态，已发货或已完成的订单不允许修改
        if order.status == "shipped" || order.status == "completed" {
            return Err(AppError::business(format!(
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
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

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
                let item_tax = item_after_discount * (tax_pct / rust_decimal::Decimal::new(100, 0));
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
                .ok_or_else(|| AppError::business("销售订单不存在"))?;
            let mut order_update: sales_order::ActiveModel = order_entity.into();
            order_update.subtotal = sea_orm::ActiveValue::Set(subtotal);
            order_update.tax_amount = sea_orm::ActiveValue::Set(tax_amount);
            order_update.discount_amount = sea_orm::ActiveValue::Set(discount_amount);
            order_update.total_amount = sea_orm::ActiveValue::Set(total_amount);
            order_update.balance_amount = sea_orm::ActiveValue::Set(total_amount);
            order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                order_update,
                Some(0),
            )
            .await?;
        }

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(order_id).await
    }

    /// 删除销售订单
    pub async fn delete_order(&self, order_id: i32) -> Result<(), AppError> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 检查订单状态，已发货或已完成的订单不允许删除
        if order.status == "shipped" || order.status == "completed" {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许删除",
                order.status
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 释放预留库存
        self.release_reservations(order_id, &txn).await?;

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

    // 生成订单号
    crate::impl_generate_no!(
        generate_order_no,
        "SO",
        SalesOrderEntity,
        sales_order::Column::OrderNo
    );

    /// 检查库存是否充足
    async fn check_inventory(
        &self,
        items: &[SalesOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
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
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足，当前库存：{}，需要：{}",
                        item.product_id, s.quantity_available, item.quantity
                    )));
                }
                None => {
                    return Err(AppError::business(format!(
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
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 为每个订单明细项创建库存预留记录
        for item in items {
            // 检查是否已经存在该订单产品的预留记录
            let existing_reservation = InventoryReservationEntity::find()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
                .filter(inventory_reservation::Column::Status.eq("pending"))
                .one(txn)
                .await?;

            if existing_reservation.is_some() {
                // 已经存在预留记录，跳过
                tracing::info!("产品 {} 已存在预留记录，跳过创建", item.product_id);
                continue;
            }

            // 查询产品库存（假设使用默认仓库）
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                .one(txn)
                .await?;

            if let Some(s) = stock {
                if s.quantity_available < item.quantity {
                    return Err(AppError::business(format!(
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
                    created_by: sea_orm::ActiveValue::Set(Some(user_id)),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                reservation.insert(txn).await?;

                // 更新库存可用数量
                let new_quantity_available = s.quantity_available - item.quantity;
                let stock_update = inventory_stock::ActiveModel {
                    id: sea_orm::ActiveValue::Unchanged(s.id),
                    quantity_available: sea_orm::ActiveValue::Set(new_quantity_available),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    ..Default::default()
                };
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    stock_update,
                    Some(0),
                )
                .await?;
            } else {
                return Err(AppError::business(format!(
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
    ) -> Result<(), AppError> {
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
                    return Err(AppError::business(format!(
                        "产品 {} 在仓库 {} (批次 {}) 库存不足，当前可用 {}，需要 {}",
                        item.product_id,
                        item.warehouse_id,
                        item.batch_no,
                        s.quantity_on_hand,
                        item.quantity
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
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    stock_update,
                    Some(0),
                )
                .await?;

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
                    crate::services::audit_log_service::AuditLogService::update_with_audit(
                        txn,
                        "auto_audit",
                        res_update,
                        Some(0),
                    )
                    .await?;
                }
            } else {
                return Err(AppError::business(format!(
                    "产品 {} 在仓库 {} (批次 {}) 没有库存记录，无法扣减",
                    item.product_id, item.warehouse_id, item.batch_no
                )));
            }

            // 更新订单明细的已发货数量
            let order_item = SalesOrderItemEntity::find_by_id(item.order_item_id)
                .one(txn)
                .await?;
            if let Some(oi) = order_item {
                let current_shipped = oi.shipped_quantity;
                let mut oi_update: sales_order_item::ActiveModel = oi.into();
                oi_update.shipped_quantity =
                    sea_orm::ActiveValue::Set(current_shipped + item.quantity);
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    oi_update,
                    Some(0),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// 释放订单的库存预留记录
    async fn release_reservations(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let reservations = InventoryReservationEntity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq("pending"))
            .all(txn)
            .await?;

        for res in reservations {
            // 恢复库存可用数量
            let stock = InventoryStockEntity::find()
                .filter(inventory_stock::Column::ProductId.eq(res.product_id))
                .filter(inventory_stock::Column::WarehouseId.eq(res.warehouse_id))
                .one(txn)
                .await?;

            if let Some(s) = stock {
                let new_quantity_available = s.quantity_available + res.quantity;
                let stock_update = inventory_stock::ActiveModel {
                    id: sea_orm::ActiveValue::Unchanged(s.id),
                    quantity_available: sea_orm::ActiveValue::Set(new_quantity_available),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    ..Default::default()
                };
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    stock_update,
                    Some(0),
                )
                .await?;
            }

            let mut res_update: inventory_reservation::ActiveModel = res.into();
            res_update.status = sea_orm::ActiveValue::Set("cancelled".to_string());
            res_update.released_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
            res_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                txn,
                "auto_audit",
                res_update,
                Some(0),
            )
            .await?;
        }
        Ok(())
    }

    /// 提交销售订单
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        // 1. 查询订单
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 2. 检查状态 - 只有草稿状态可以提交
        if order.status != "draft" {
            return Err(AppError::business(format!(
                "订单状态为{}，只有草稿状态的订单可以提交",
                order.status
            )));
        }

        // 3. 开启事务
        let txn = (*self.db).begin().await?;

        // 4. 更新状态为 pending_approval
        let mut order_update: sales_order::ActiveModel = order.clone().into();
        order_update.status = sea_orm::ActiveValue::Set("pending_approval".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        // 5. 挂载 BPM 引擎
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());

        // 获取用户信息
        let user_service = UserService::new(self.db.clone());
        let user = user_service.find_by_id(user_id).await?;

        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "sales_order_approval".to_string(),
            business_type: "sales_order".to_string(),
            business_id: order_id,
            title: format!("销售订单审批 - {}", order.order_no),
            initiator_id: user_id,
            initiator_name: user.username,
            initiator_department_id: None,
            priority: None,
            form_data: None,
            variables: None,
        };
        // 忽略找不到模板的错误，为了兼容旧数据
        let _ = bpm_service.start_process(req).await;

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(order_id).await
    }

    /// 审核销售订单
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 检查订单状态，只有待审批状态可以审核
        if order.status != "pending_approval" {
            return Err(AppError::business(format!(
                "订单状态为{}，只有待审批状态的订单可以审核",
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

        self.lock_inventory(order_id, &items, user_id, &txn).await?;

        // 更新订单状态为已审核
        let updated_order = sales_order::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(order.id),
            status: sea_orm::ActiveValue::Set("approved".to_string()),
            approved_by: sea_orm::ActiveValue::Set(Some(user_id)), // 使用实际审批人
            approved_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now())),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let approved_order =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                updated_order,
                Some(0),
            )
            .await?;

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(approved_order.id).await
    }

    /// 发货处理
    pub async fn ship_order(
        &self,
        order_id: i32,
        req: ShipOrderRequest,
    ) -> Result<SalesOrderDetail, AppError> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 检查订单状态，只有已审核状态可以发货
        if order.status != "approved" {
            return Err(AppError::business(format!(
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

        let shipped_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            updated_order,
            Some(0),
        )
        .await?;

        // 自动生成应收账款单据
        let invoice_no = DocumentNumberGenerator::generate_no(
            &*self.db,
            "AR",
            ArInvoiceEntity,
            ArInvoiceColumn::InvoiceNo,
        )
        .await
        .map_err(|e| AppError::business(e.to_string()))?;

        // 查询客户信息
        let customer = customer::Entity::find_by_id(order.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::business(format!("客户 {} 不存在", order.customer_id)))?;

        let invoice_date = chrono::Utc::now().date_naive();
        let due_date = invoice_date + chrono::Duration::days(30);

        let ar_invoice = ar_invoice::ActiveModel {
            invoice_no: sea_orm::ActiveValue::Set(invoice_no),
            invoice_date: sea_orm::ActiveValue::Set(invoice_date),
            due_date: sea_orm::ActiveValue::Set(due_date),
            customer_id: sea_orm::ActiveValue::Set(order.customer_id),
            customer_name: sea_orm::ActiveValue::Set(Some(customer.customer_name.clone())),
            source_type: sea_orm::ActiveValue::Set(Some("SALES_ORDER".to_string())),
            source_bill_id: sea_orm::ActiveValue::Set(Some(order_id)),
            source_bill_no: sea_orm::ActiveValue::Set(Some(order.order_no.clone())),
            invoice_amount: sea_orm::ActiveValue::Set(order.total_amount),
            received_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            unpaid_amount: sea_orm::ActiveValue::Set(order.total_amount),
            status: sea_orm::ActiveValue::Set("APPROVED".to_string()),
            approval_status: sea_orm::ActiveValue::Set("APPROVED".to_string()),
            created_by: sea_orm::ActiveValue::Set(0), // 系统生成
            ..Default::default()
        };

        ar_invoice.insert(&txn).await?;

        // 提交事务
        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(shipped_order.id).await
    }

    /// 完成订单
    pub async fn complete_order(&self, order_id: i32) -> Result<SalesOrderDetail, AppError> {
        // 检查订单是否存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 检查订单状态，只有已发货状态可以完成
        if order.status != "shipped" {
            return Err(AppError::business(format!(
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

        let completed_order =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                updated_order,
                Some(0),
            )
            .await?;

        // 提交事务
        txn.commit().await?;

        // 订单完成后更新商机状态
        if let Some(opportunity_id) = order.opportunity_id {
            let crm_service = crate::services::crm_service::CrmService::new(self.db.clone());
            if let Err(e) = crm_service
                .update_opportunity_on_order_complete(opportunity_id, order.total_amount)
                .await
            {
                tracing::error!("更新商机状态失败 (商机 {}): {}", opportunity_id, e);
            } else {
                tracing::info!("成功更新商机状态为已成交 (商机 {})", opportunity_id);
            }
        }

        // 返回订单详情
        self.get_order_detail(completed_order.id).await
    }

    /// 拒绝销售订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
    ) -> Result<SalesOrderDetail, AppError> {
        // 1. 查询订单
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 2. 检查状态
        if order.status != "draft" && order.status != "pending_approval" {
            return Err(AppError::business(format!(
                "订单状态为{}，只有草稿或待审批状态的订单可以拒绝",
                order.status
            )));
        }

        // 3. 开启事务，释放预留库存
        let txn = (*self.db).begin().await?;
        self.release_reservations(order_id, &txn).await?;

        // 4. 更新状态
        let updated_order = sales_order::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(order.id),
            status: sea_orm::ActiveValue::Set("rejected".to_string()),
            notes: sea_orm::ActiveValue::Set(Some(reason)),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let rejected_order =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                updated_order,
                Some(0),
            )
            .await?;

        txn.commit().await?;

        // 返回订单详情
        self.get_order_detail(rejected_order.id).await
    }

    // ========== 数据导出方法 ==========

    /// 导出销售订单为 CSV 格式
    pub async fn export_orders_to_csv(
        &self,
        status: Option<String>,
        customer_id: Option<i32>,
        order_no: Option<String>,
    ) -> Result<Vec<u8>, AppError> {
        let page_req = PageRequest {
            page: 1,
            page_size: 10000,
        };
        let orders = self
            .list_orders(page_req, status, customer_id, order_no)
            .await?;

        let headers = vec![
            "订单编号".to_string(),
            "客户ID".to_string(),
            "客户名称".to_string(),
            "商机ID".to_string(),
            "订单日期".to_string(),
            "要求交货日期".to_string(),
            "发货日期".to_string(),
            "状态".to_string(),
            "小计金额".to_string(),
            "税额".to_string(),
            "折扣金额".to_string(),
            "运费".to_string(),
            "总金额".to_string(),
            "已付金额".to_string(),
            "余额".to_string(),
            "送货地址".to_string(),
            "账单地址".to_string(),
            "备注".to_string(),
            "创建人ID".to_string(),
            "审批人ID".to_string(),
            "审批时间".to_string(),
        ];

        let rows: Vec<std::collections::HashMap<String, String>> = orders
            .items
            .into_iter()
            .map(|o| {
                let mut row = std::collections::HashMap::new();
                row.insert("订单编号".to_string(), o.order_no);
                row.insert("客户ID".to_string(), o.customer_id.to_string());
                row.insert("客户名称".to_string(), o.customer_name.unwrap_or_default());
                row.insert(
                    "商机ID".to_string(),
                    o.opportunity_id
                        .map(|id| id.to_string())
                        .unwrap_or_default(),
                );
                row.insert(
                    "订单日期".to_string(),
                    o.order_date.format("%Y-%m-%d %H:%M:%S").to_string(),
                );
                row.insert(
                    "要求交货日期".to_string(),
                    o.required_date.format("%Y-%m-%d %H:%M:%S").to_string(),
                );
                row.insert(
                    "发货日期".to_string(),
                    o.ship_date
                        .map(|d: chrono::DateTime<chrono::Utc>| {
                            d.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                );
                row.insert("状态".to_string(), o.status);
                row.insert("小计金额".to_string(), o.subtotal.to_string());
                row.insert("税额".to_string(), o.tax_amount.to_string());
                row.insert("折扣金额".to_string(), o.discount_amount.to_string());
                row.insert("运费".to_string(), o.shipping_cost.to_string());
                row.insert("总金额".to_string(), o.total_amount.to_string());
                row.insert("已付金额".to_string(), o.paid_amount.to_string());
                row.insert("余额".to_string(), o.balance_amount.to_string());
                row.insert(
                    "送货地址".to_string(),
                    o.shipping_address.unwrap_or_default(),
                );
                row.insert(
                    "账单地址".to_string(),
                    o.billing_address.unwrap_or_default(),
                );
                row.insert("备注".to_string(), o.notes.unwrap_or_default());
                row.insert(
                    "创建人ID".to_string(),
                    o.created_by
                        .map(|id: i32| id.to_string())
                        .unwrap_or_default(),
                );
                row.insert(
                    "审批人ID".to_string(),
                    o.approved_by
                        .map(|id: i32| id.to_string())
                        .unwrap_or_default(),
                );
                row.insert(
                    "审批时间".to_string(),
                    o.approved_at
                        .map(|d: chrono::DateTime<chrono::Utc>| {
                            d.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                );
                row
            })
            .collect();

        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::business(format!("CSV 生成失败: {}", e)))
    }

    // ========== 订单状态操作方法 ==========

    /// 取消订单
    pub async fn cancel_order(
        &self,
        order_id: i32,
        _user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        // 获取订单
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        // 检查订单状态是否允许取消
        if !["draft", "pending", "approved"].contains(&order.status.as_str()) {
            return Err(AppError::business("当前状态不允许取消".to_string()));
        }

        // 更新订单状态
        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("cancelled".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        order_update.update(&*self.db).await?;

        self.get_order_detail(order_id).await
    }

    // ========== 发货记录方法 ==========

    /// 获取订单发货记录
    pub async fn get_order_deliveries(
        &self,
        _order_id: i32,
        _page: u64,
        _page_size: u64,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        // 简化实现：返回空列表
        Ok((vec![], 0))
    }

    /// 创建发货
    pub async fn create_delivery(
        &self,
        order_id: i32,
        _payload: serde_json::Value,
        _user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 验证订单存在
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        // 检查订单状态
        if !["approved", "processing"].contains(&order.status.as_str()) {
            return Err(AppError::business("当前状态不允许发货".to_string()));
        }

        // 简化实现：返回模拟的发货记录
        Ok(serde_json::json!({
            "id": 0,
            "order_id": order_id,
            "status": "pending",
            "delivery_date": chrono::Utc::now(),
            "notes": null,
        }))
    }

    // ========== 统计方法 ==========

    /// 获取订单统计
    pub async fn get_order_statistics(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        use sea_orm::QuerySelect;

        let _start_date = query
            .get("start_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2020-01-01");

        let _end_date = query
            .get("end_date")
            .and_then(|v| v.as_str())
            .unwrap_or("2099-12-31");

        let customer_id = query
            .get("customer_id")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let mut query = SalesOrderEntity::find()
            .select_only()
            .column_as(sales_order::Column::Id.count(), "total_orders")
            .column_as(sales_order::Column::TotalAmount.sum(), "total_amount");

        if let Some(cid) = customer_id {
            query = query.filter(sales_order::Column::CustomerId.eq(cid));
        }

        let result = query
            .into_model::<serde_json::Value>()
            .one(&*self.db)
            .await?;

        Ok(result.unwrap_or_else(|| {
            serde_json::json!({
                "total_orders": 0,
                "total_amount": 0,
                "completed_orders": 0,
                "cancelled_orders": 0,
                "pending_orders": 0,
                "approved_orders": 0,
            })
        }))
    }
}
