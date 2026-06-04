//! 销售订单核心服务（so/order）
//!
//! 包含销售订单的 CRUD、生命周期、查询、统计等。
//! 拆分自原 `sales_service.rs`。

use crate::models::dto::PageRequest;
use crate::models::{
    ar_invoice::{self, Column as ArInvoiceColumn, Entity as ArInvoiceEntity},
    customer, product, sales_order,
    sales_order::Entity as SalesOrderEntity,
    sales_order_item,
    sales_order_item::Entity as SalesOrderItemEntity,
};
use crate::services::so::{
    CreateSalesOrderRequest, SalesOrderDetail, SalesOrderItemDetail, UpdateSalesOrderRequest,
};
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, TransactionTrait,
};
use std::sync::Arc;

/// 销售订单服务
pub struct SalesService {
    pub(crate) db: Arc<DatabaseConnection>,
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
            .join(
                sea_orm::JoinType::LeftJoin,
                sales_order::Relation::Customer.def(),
            );

        if let Some(s) = status {
            query = query.filter(sales_order::Column::Status.eq(s));
        }
        if let Some(cid) = customer_id {
            query = query.filter(sales_order::Column::CustomerId.eq(cid));
        }
        if let Some(no) = order_no {
            query = query.filter(sales_order::Column::OrderNo.contains(&no));
        }

        let paginator = query
            .order_by(sales_order::Column::CreatedAt, Order::Desc)
            .into_model::<SalesOrderDetail>()
            .paginate(&*self.db, page_req.page_size);

        let total = paginator.num_items().await?;
        let mut order_details: Vec<SalesOrderDetail> =
            paginator.fetch_page(page_req.page - 1).await?;

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
        let mut order = SalesOrderEntity::find_by_id(order_id)
            .column_as(
                crate::models::customer::Column::CustomerName,
                "customer_name",
            )
            .join(
                sea_orm::JoinType::LeftJoin,
                sales_order::Relation::Customer.def(),
            )
            .into_model::<SalesOrderDetail>()
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        let items = SalesOrderItemEntity::find()
            .column_as(product::Column::Code, "product_code")
            .column_as(product::Column::Name, "product_name")
            .join(
                sea_orm::JoinType::LeftJoin,
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
        let customer = customer::Entity::find_by_id(request.customer_id)
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

        // 检查库存并预留
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

            let item_subtotal = item_req.quantity * item_req.unit_price;
            let item_discount = item_subtotal * (discount_pct / rust_decimal::Decimal::new(100, 0));
            let item_after_discount = item_subtotal - item_discount;
            let item_tax = item_after_discount * (tax_pct / rust_decimal::Decimal::new(100, 0));
            let item_total = item_after_discount + item_tax;

            subtotal += &item_subtotal;
            discount_amount += &item_discount;
            tax_amount += &item_tax;
            total_amount += &item_total;

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

        // 占用信用额度
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
        if let Ok(Some(warning)) = credit_service
            .check_credit_warning(request.customer_id)
            .await
        {
            tracing::warn!("信用预警: {}", warning);
        }

        // 订单回写商机
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

        // 检查订单状态
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
            SalesOrderItemEntity::delete_many()
                .filter(sales_order_item::Column::OrderId.eq(order_id))
                .exec(&txn)
                .await?;

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

        // 检查订单状态
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

    // ========== 库存辅助方法（私有） ==========
    // 注意：lock_inventory、reduce_inventory、release_reservations、check_inventory
    // 已迁移到 so/delivery.rs，避免重复实现

    // ========== 数据导出方法 ==========
    // 注意：export_orders_to_csv 已迁移到 so/delivery.rs

    // ========== 订单生命周期方法（handler 调用） ==========

    /// 提交订单：草稿 -> 提交审批（含信用检查、BPM 启动）
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != "draft" {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法提交",
                order.status
            )));
        }

        // 客户信用度复检
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let total_amount_bigdecimal = {
            use bigdecimal::BigDecimal;
            BigDecimal::parse_bytes(order.total_amount.to_string().as_bytes(), 10)
                .unwrap_or_else(|| BigDecimal::from(0))
        };
        let credit_available = credit_service
            .check_credit_available(order.customer_id, total_amount_bigdecimal)
            .await
            .map_err(|e| AppError::business(format!("信用检查失败: {}", e)))?;
        if !credit_available {
            return Err(AppError::business("信用额度不足，无法提交订单"));
        }

        // 客户状态校验
        let customer = crate::models::customer::Entity::find_by_id(order.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;
        if customer.status != "active" {
            return Err(AppError::business(format!(
                "客户状态为 {}，不允许提交订单",
                customer.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("pending".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        // 启动审批工作流（BPM）
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let _ = bpm_service
            .start_process(crate::models::dto::bpm_dto::StartProcessRequest {
                process_key: "sales_order_approval".to_string(),
                business_type: "sales_order".to_string(),
                business_id: order_id,
                title: format!("销售订单审批 - {}", order.order_no),
                initiator_id: user_id,
                initiator_name: String::new(),
                initiator_department_id: None,
                priority: None,
                form_data: None,
                variables: None,
            })
            .await;

        Ok(order)
    }

    /// 审核订单：通过或拒绝
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != "pending" {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法审核",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("approved".to_string());
        order_update.approved_by = sea_orm::ActiveValue::Set(Some(user_id));
        order_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        Ok(order)
    }

    /// 完成订单
    pub async fn complete_order(&self, order_id: i32) -> Result<sales_order::Model, AppError> {
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != "shipped" {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法完成",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        Ok(order)
    }
}

// 解决 ar_invoice 引用告警
#[allow(dead_code)]
type _Ar = ArInvoiceEntity;
#[allow(dead_code)]
type _ArCol = ArInvoiceColumn;
