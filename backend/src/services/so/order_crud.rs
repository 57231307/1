//! 销售订单 CRUD 子模块（order_crud）
//!
//! P9-2 拆分自原 `services/so/order.rs`。
//! 包含：create_order / update_order / delete_order
//!
//! ## 模块职责
//! - 销售订单创建（含事务、订单号生成、库存锁定、信用校验）
//! - 销售订单更新（订单头 + 明细项）
//! - 销售订单删除
//!
//! ## API 兼容
//! 通过 `crate::services::so::order::SalesService` 路径访问。

use super::order::SalesService;
use crate::models::{
    ar_invoice::{self},
    customer, product, sales_order,
    sales_order::Entity as SalesOrderEntity,
    sales_order_item,
    sales_order_item::Entity as SalesOrderItemEntity,
};
use crate::models::status::sales_order as so_status;
use crate::search::{SalesOrderDoc, SalesOrderItemDoc};
use crate::services::so::{
    CreateSalesOrderRequest, SalesOrderDetail, UpdateSalesOrderRequest,
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
};

/// 销售订单 CRUD 子模块标记
pub const P92_CRUD_MODULE: &str = "sales_order_crud";

/// Decimal → f64 转换工具函数
///
/// 批次 125 v8 复审 P1 修复：ES 索引字段为 f64，PG Decimal 需转换。
/// 使用 to_string().parse() 避免精度损失（Decimal::to_f64 在某些边界值会丢精度）。
fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

/// 订单金额累计：用于 create_order 拆分时在 helper 间传递总额
struct OrderTotals {
    subtotal: rust_decimal::Decimal,
    tax_amount: rust_decimal::Decimal,
    discount_amount: rust_decimal::Decimal,
    total_amount: rust_decimal::Decimal,
}

impl SalesService {
    // create_order / update_order / delete_order
    // 内容来自原 order.rs L277-610 + L611-777 + L778-814

    /// 将 SalesOrderDetail 转换为 SalesOrderDoc 用于 ES 索引
    ///
    /// 批次 125 v8 复审 P1 修复：字段映射规则
    /// - total_amount: Decimal → f64（decimal.to_string().parse::<f64>().unwrap_or(0.0)）
    /// - items: 从 SalesOrderItemDetail 构建 SalesOrderItemDoc（quantity/unit_price Decimal→f64）
    /// - items.color_no: String → Option<String>（空字符串转 None）
    fn build_sales_order_doc(detail: &SalesOrderDetail) -> SalesOrderDoc {
        let items: Vec<SalesOrderItemDoc> = detail
            .items
            .iter()
            .map(|item| SalesOrderItemDoc {
                product_id: item.product_id,
                product_name: item.product_name.clone().unwrap_or_default(),
                quantity: decimal_to_f64(&item.quantity),
                unit_price: decimal_to_f64(&item.unit_price),
                color_no: if item.color_no.is_empty() {
                    None
                } else {
                    Some(item.color_no.clone())
                },
                pantone_code: item.pantone_code.clone(),
            })
            .collect();

        SalesOrderDoc {
            order_no: detail.order_no.clone(),
            customer_id: detail.customer_id,
            customer_name: detail.customer_name.clone().unwrap_or_default(),
            total_amount: decimal_to_f64(&detail.total_amount),
            status: detail.status.clone(),
            created_at: detail.created_at,
            items,
        }
    }

    /// 同步销售订单到 ES（最终一致性策略）
    ///
    /// 批次 125 v8 复审 P1 修复：ES 同步失败仅记录日志，不回滚 PG 事务。
    async fn sync_sales_order_to_es(&self, detail: &SalesOrderDetail, operation: &str) {
        let doc = Self::build_sales_order_doc(detail);
        if let Err(e) = self.search_syncer.sync_sales_order(&doc).await {
            tracing::warn!(
                error = %e,
                order_id = detail.id,
                order_no = %detail.order_no,
                operation = operation,
                "ES 销售订单同步失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }
    }

    pub async fn create_order(
        &self,
        request: CreateSalesOrderRequest,
        user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        let customer_id = request.customer_id;
        let opportunity_id = request.opportunity_id;
        let txn = (*self.db).begin().await?;
        let required_date = self.validate_create_preconditions(&request, &txn).await?;
        let order_amount = Self::calculate_order_amount(&request);
        self.check_credit_available(customer_id, &txn, order_amount)
            .await?;
        let order_no = self.generate_unique_order_no(&txn).await?;
        let order_entity = self
            .create_order_main_record(&request, required_date, order_no, &txn)
            .await?;
        self.lock_inventory(order_entity.id, &request.items, user_id, &txn)
            .await?;
        Self::validate_products_exist(&request, &txn).await?;
        let totals = self
            .create_order_items_and_calculate_totals(request.items, order_entity.id, &txn)
            .await?;
        let order_id = self
            .update_order_totals(order_entity, totals, user_id, &txn)
            .await?;
        self.occupy_credit_and_warn(customer_id, order_amount, user_id)
            .await?;
        self.writeback_opportunity(opportunity_id, totals.total_amount, &txn)
            .await?;
        txn.commit().await?;
        // 返回订单详情（service 内部调用，无数据权限过滤）
        let detail = self.get_order_detail(order_id, None).await?;
        // 批次 125 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        self.sync_sales_order_to_es(&detail, "create").await;
        Ok(detail)
    }

    /// 创建订单前置校验：客户存在性 + 日期合理性
    async fn validate_create_preconditions(
        &self,
        request: &CreateSalesOrderRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
        // 业务逻辑验证：检查客户是否存在
        let customer = customer::Entity::find_by_id(request.customer_id)
            .one(txn)
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
                .all(txn)
                .await;
            match unpaid_result {
                Ok(invoices) => invoices.iter().map(|i| i.invoice_amount).sum(),
                Err(_) => rust_decimal::Decimal::ZERO,
            }
        };
        Ok(required_date)
    }

    /// 计算订单金额（纯函数，无 IO）
    fn calculate_order_amount(request: &CreateSalesOrderRequest) -> rust_decimal::Decimal {
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
        order_amount
    }

    /// 信用额度校验：调用 CustomerCreditService 检查可用额度
    async fn check_credit_available(
        &self,
        customer_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        order_amount: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        // 使用信用服务检查额度
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let credit_available = credit_service
            .check_credit_available(customer_id, order_amount)
            .await
            .map_err(|err| AppError::business(format!("信用检查失败: {}", err)))?;
        if !credit_available {
            tracing::error!("Transaction rolled back: 信用额度不足");
            txn.rollback().await?;
            return Err(AppError::business(format!(
                "信用额度不足：订单金额 {} 超出可用信用额度",
                order_amount
            )));
        }
        Ok(())
    }

    /// 生成订单号并校验唯一性（防并发冲突）
    async fn generate_unique_order_no(
        &self,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<String, AppError> {
        // 生成订单号并检查唯一性
        let order_no = self.generate_order_no().await?;
        // 再次检查订单号是否已存在（防止并发冲突）
        let existing_order = SalesOrderEntity::find()
            .filter(sales_order::Column::OrderNo.eq(&order_no))
            .one(txn)
            .await?;
        if existing_order.is_some() {
            tracing::error!("Transaction rolled back: 订单号 {} 已存在", order_no);
            txn.rollback().await?;
            return Err(AppError::business("订单号已存在，请重试"));
        }
        Ok(order_no)
    }

    /// 创建订单主表（初始金额为 0，后续由 update_order_totals 更新）
    async fn create_order_main_record(
        &self,
        request: &CreateSalesOrderRequest,
        required_date: chrono::DateTime<chrono::Utc>,
        order_no: String,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<sales_order::Model, AppError> {
        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: sea_orm::ActiveValue::Set(order_no),
            customer_id: sea_orm::ActiveValue::Set(request.customer_id),
            opportunity_id: sea_orm::ActiveValue::Set(request.opportunity_id),
            order_date: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            required_date: sea_orm::ActiveValue::Set(required_date),
            ship_date: sea_orm::ActiveValue::NotSet,
            status: sea_orm::ActiveValue::Set(
                request
                    .status
                    .clone()
                    .unwrap_or_else(|| so_status::DRAFT.to_string()),
            ),
            subtotal: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            tax_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            discount_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            shipping_cost: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            total_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            paid_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            balance_amount: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            shipping_address: sea_orm::ActiveValue::Set(request.shipping_address.clone()),
            billing_address: sea_orm::ActiveValue::Set(request.billing_address.clone()),
            notes: sea_orm::ActiveValue::Set(request.notes.clone()),
            created_by: sea_orm::ActiveValue::NotSet,
            approved_by: sea_orm::ActiveValue::NotSet,
            approved_at: sea_orm::ActiveValue::NotSet,
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };
        Ok(order.insert(txn).await?)
    }

    /// 产品存在性批量校验（防订单明细引用不存在的产品）
    async fn validate_products_exist(
        request: &CreateSalesOrderRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 验证产品是否存在（批量查询优化）
        let mut product_ids = std::collections::HashSet::new();
        for item in &request.items {
            product_ids.insert(item.product_id);
        }
        if product_ids.is_empty() {
            return Ok(());
        }
        let existing_products = product::Entity::find()
            .filter(product::Column::Id.is_in(product_ids.iter().cloned().collect::<Vec<_>>()))
            .all(txn)
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
        Ok(())
    }

    /// 创建订单明细 + 累计金额（批量 INSERT 优化）
    async fn create_order_items_and_calculate_totals(
        &self,
        items: Vec<crate::services::so::SalesOrderItemRequest>,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<OrderTotals, AppError> {
        let mut subtotal = rust_decimal::Decimal::ZERO;
        let mut tax_amount = rust_decimal::Decimal::ZERO;
        let mut discount_amount = rust_decimal::Decimal::ZERO;
        let mut total_amount = rust_decimal::Decimal::ZERO;
        let mut item_models = Vec::new();
        for item_req in items {
            let discount_pct = item_req
                .discount_percent
                .unwrap_or(rust_decimal::Decimal::ZERO);
            let tax_pct = item_req.tax_percent.unwrap_or(rust_decimal::Decimal::ZERO);
            // 批次 97 P1-6 修复（v5 复审）：金额计算补 round_dp(2) 防止精度漂移
            let item_subtotal = (item_req.quantity * item_req.unit_price).round_dp(2);
            let item_discount =
                (item_subtotal * (discount_pct / rust_decimal::Decimal::new(100, 0))).round_dp(2);
            let item_after_discount = (item_subtotal - item_discount).round_dp(2);
            let item_tax =
                (item_after_discount * (tax_pct / rust_decimal::Decimal::new(100, 0))).round_dp(2);
            let item_total = (item_after_discount + item_tax).round_dp(2);

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
            item_models.push(item);
        }
        if !item_models.is_empty() {
            sales_order_item::Entity::insert_many(item_models)
                .exec(txn)
                .await?;
        }
        Ok(OrderTotals {
            subtotal,
            tax_amount,
            discount_amount,
            total_amount,
        })
    }

    /// 更新订单总金额（含审计日志）
    async fn update_order_totals(
        &self,
        order_entity: sales_order::Model,
        totals: OrderTotals,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<i32, AppError> {
        // 更新订单总金额
        let order_id = order_entity.id;
        let mut order_update: sales_order::ActiveModel = order_entity.into();
        order_update.subtotal = sea_orm::ActiveValue::Set(totals.subtotal);
        order_update.tax_amount = sea_orm::ActiveValue::Set(totals.tax_amount);
        order_update.discount_amount = sea_orm::ActiveValue::Set(totals.discount_amount);
        order_update.total_amount = sea_orm::ActiveValue::Set(totals.total_amount);
        order_update.balance_amount = sea_orm::ActiveValue::Set(totals.total_amount);
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_update,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;
        Ok(order_id)
    }

    /// 占用信用额度 + 信用预警检查
    async fn occupy_credit_and_warn(
        &self,
        customer_id: i32,
        order_amount: rust_decimal::Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 占用信用额度
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        credit_service
            .occupy_credit(customer_id, order_amount, user_id)
            .await
            .map_err(|e| {
                tracing::error!("信用额度占用失败，事务回滚: {}", e);
                AppError::business(format!("信用额度占用失败: {}", e))
            })?;
        tracing::info!(
            "客户 {} 信用额度占用成功，金额: {}",
            customer_id,
            order_amount
        );
        if let Ok(Some(warning)) = credit_service
            .check_credit_warning(customer_id)
            .await
        {
            tracing::warn!("信用预警: {}", warning);
        }
        Ok(())
    }

    /// 订单回写商机（actual_amount / actual_close_date / stage=closed_won）
    async fn writeback_opportunity(
        &self,
        opportunity_id: Option<i32>,
        total_amount: rust_decimal::Decimal,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 订单回写商机
        let Some(opportunity_id) = opportunity_id else {
            return Ok(());
        };
        use crate::models::crm_opportunity;
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business(format!("商机 {} 不存在", opportunity_id)))?;
        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.actual_amount = sea_orm::ActiveValue::Set(Some(total_amount));
        opp_active.actual_close_date =
            sea_orm::ActiveValue::Set(Some(chrono::Utc::now().date_naive()));
        opp_active.opportunity_stage = sea_orm::ActiveValue::Set(Some("closed_won".to_string()));
        opp_active.opportunity_status = sea_orm::ActiveValue::Set(Some("won".to_string()));
        opp_active.updated_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        opp_active.update(txn).await?;
        tracing::info!(
            "商机 {} 已关联订单并更新实际金额: {}",
            opportunity_id,
            total_amount
        );
        Ok(())
    }

    /// 更新销售订单
    ///
    /// 批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，
    /// 保证订单更新审计日志能追溯实际操作人。
    pub async fn update_order(
        &self,
        order_id: i32,
        user_id: i32,
        request: UpdateSalesOrderRequest,
    ) -> Result<SalesOrderDetail, AppError> {
        // 批次 18（2026-06-28）：状态门查询移入事务内并加 lock_exclusive。
        // 原实现先在事务外用 &*self.db 查询订单状态，再 begin() 开启事务，
        // 并发 update_order 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;

        // 检查订单是否存在（加 lock_exclusive 串行化并发修改，防止基于过期状态写入）
        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 检查订单状态
        if order.status == so_status::SHIPPED || order.status == so_status::COMPLETED {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许修改",
                order.status
            )));
        }

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
        // 批次 94 P2-10：原 Some(0) 占位符改为真实操作人 user_id（P3 3-27 TODO 已解决）
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
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

            let mut item_models = Vec::new();
            for item_req in items {
                let discount_pct = item_req
                    .discount_percent
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let tax_pct = item_req.tax_percent.unwrap_or(rust_decimal::Decimal::ZERO);

                // 批次 97 P1-6 修复（v5 复审）：金额计算补 round_dp(2) 防止精度漂移
                let item_subtotal = (item_req.quantity * item_req.unit_price).round_dp(2);
                let item_discount =
                    (item_subtotal * (discount_pct / rust_decimal::Decimal::new(100, 0))).round_dp(2);
                let item_after_discount = (item_subtotal - item_discount).round_dp(2);
                let item_tax = (item_after_discount * (tax_pct / rust_decimal::Decimal::new(100, 0))).round_dp(2);
                let item_total = (item_after_discount + item_tax).round_dp(2);

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

                item_models.push(item);
            }

            if !item_models.is_empty() {
                sales_order_item::Entity::insert_many(item_models)
                    .exec(&txn)
                    .await?;
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
            // 批次 94 P2-10：原 Some(0) 占位符改为真实操作人 user_id
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                order_update,
                Some(user_id),
            )
            .await?;
        }

        // 提交事务
        txn.commit().await?;

        // 返回订单详情（service 内部调用，无数据权限过滤）
        let detail = self.get_order_detail(order_id, None).await?;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后同步到 ES（最终一致性）
        self.sync_sales_order_to_es(&detail, "update").await;

        Ok(detail)
    }

    /// 删除销售订单
    ///
    /// 批次 94 P2-10：补 user_id 参数，将 Some(0) 占位符改为真实操作人 user_id，
    /// 保证订单删除审计日志能追溯实际操作人。
    pub async fn delete_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 &*self.db 裸查询订单状态，再 begin() 开启事务，
        // 并发 delete_order 均通过状态检查后基于过期状态删除，导致状态门失效。
        let txn = (*self.db).begin().await?;

        // 检查订单是否存在（加 lock_exclusive 串行化并发状态变更）
        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 未找到", order_id)))?;

        // 批次 125 v8 复审 P1 修复：删除前保存 order_no 用于 ES 文档删除
        let order_no_for_es = order.order_no.clone();

        // 检查订单状态
        if order.status == so_status::SHIPPED || order.status == so_status::COMPLETED {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许删除",
                order.status
            )));
        }

        // 释放预留库存
        self.release_reservations(order_id, &txn).await?;

        // 删除订单明细项
        SalesOrderItemEntity::delete_many()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .exec(&txn)
            .await?;

        // 删除订单主表（P0 8-3 修复：补审计日志）
        // 批次 94 P2-10：原 Some(0) 占位符改为真实操作人 user_id
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            SalesOrderEntity,
            _,
        >(&txn, "sales_order", order_id, Some(user_id))
        .await?;

        // 提交事务
        txn.commit().await?;

        // 批次 125 v8 复审 P1 修复：PG 事务提交后删除 ES 文档（最终一致性）
        // 销售订单是硬删除，ES 文档也需删除（与客户软删除不同）
        if let Err(e) = self.search_syncer.delete_sales_order(&order_no_for_es).await {
            tracing::warn!(
                error = %e,
                order_id = order_id,
                order_no = %order_no_for_es,
                operation = "delete",
                "ES 销售订单删除失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crud_module_loaded() {
        assert_eq!(P92_CRUD_MODULE, "sales_order_crud");
    }
}
