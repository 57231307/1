//! 销售发货-发货主流程子模块（delivery_ops/ship）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L126-694 迁移。
//! 包含 ship_order 及其 15 个辅助方法：
//! - ship_order（公开 API）
//! - validate_ship_preconditions / load_ship_order_context / create_shipment_delivery
//! - process_shipment_items / compute_line_amounts / build_delivery_item
//! - compute_quantity_kg / build_record_transaction_args / update_order_item_shipped_qty
//! - update_order_after_shipment / post_commit_shipment_effects / check_order_fully_shipped
//! - create_revenue_voucher_for_delivery / build_revenue_voucher_request / build_revenue_voucher_item

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::{
    sales_delivery, sales_delivery_item, sales_order, sales_order_item, warehouse,
};
use crate::models::status::sales_delivery as delivery_status;
use crate::models::status::sales_order as so_status;
use crate::utils::error::AppError;

use super::super::delivery::{ShipOrderItemRequest, ShipOrderRequest};
use super::super::order::SalesService;
use super::types::{ShipOrderContext, ShipPostCommitContext, ShipmentItemsResult};

impl SalesService {
    /// 销售订单发货
    pub async fn ship_order(
        &self,
        request: ShipOrderRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        self.validate_ship_preconditions(&request).await?;
        let txn = (*self.db).begin().await?;
        let ctx = self.load_ship_order_context(&request, &txn).await?;
        let delivery = self
            .create_shipment_delivery(&request, &ctx.order, &ctx.warehouse, user_id, &txn)
            .await?;
        let items_result = self
            .process_shipment_items(&request, &ctx, &delivery, user_id, &txn)
            .await?;
        let post_ctx = self
            .update_order_after_shipment(&request, ctx, user_id, &txn)
            .await?;
        txn.commit().await?;
        self.post_commit_shipment_effects(&delivery, post_ctx, items_result, user_id)
            .await;
        Ok(())
    }

    /// 发货前置校验：缸号一致性 + 大货批色门禁
    async fn validate_ship_preconditions(
        &self,
        request: &ShipOrderRequest,
    ) -> Result<(), AppError> {
        // v14 批次 421 T-P1-5：缸号同订单校验
        // 依据：fabric-industry-research.md §2.3 约束 5 - 同一订单同面料必须使用相同缸号
        // 必须在开启事务前校验，避免无效请求占用数据库事务资源
        super::super::delivery::validate_dye_lot_consistency(&request.items)?;
        // V15 P0-F19：发货前校验大货批色门禁
        // 业务规则：销售订单关联的所有 bulk_color_approval 记录必须全部为 approved 状态
        // 否则阻止发货（delivery_blocking=true 阻断）
        crate::services::bulk_color_approval_service::validate_bulk_color_approval(
            &self.db,
            request.order_id,
        )
        .await
        .map_err(|e| match e {
            crate::services::bulk_color_approval_service::BulkColorApprovalError::InvalidState(
                msg,
            ) => AppError::business(msg),
            crate::services::bulk_color_approval_service::BulkColorApprovalError::SalesOrderNotFound => {
                AppError::not_found("销售订单不存在")
            }
            crate::services::bulk_color_approval_service::BulkColorApprovalError::Database(e) => {
                AppError::database(e.to_string())
            }
            other => AppError::business(other.to_string()),
        })?;
        Ok(())
    }

    /// 事务内加载发货上下文：订单锁定 + 明细 + 产品 + 仓库 + 库存校验
    async fn load_ship_order_context(
        &self,
        request: &ShipOrderRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ShipOrderContext, AppError> {
        // 检查订单状态（加 lock_exclusive 串行化并发发货）
        let order = sales_order::Entity::find_by_id(request.order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;
        if order.status != so_status::APPROVED {
            return Err(AppError::business("只有已审批的订单才能发货"));
        }
        // 查询订单明细
        // F-P0-6 修复（批次 382 v13 复审）：保留查询结果用于计算发货金额
        let order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(txn)
            .await?;
        // v14 批次 418 修复 G-P0-1：批量查询产品获取 gram_weight/width，
        // 用于库存流水的 quantity_kg 双单位换算（替代原 Decimal::ZERO 硬编码）
        let product_ids: Vec<i32> = order_items.iter().map(|oi| oi.product_id).collect();
        let products = if product_ids.is_empty() {
            Vec::new()
        } else {
            crate::models::product::Entity::find()
                .filter(crate::models::product::Column::Id.is_in(product_ids))
                .all(txn)
                .await?
        };
        let product_map: std::collections::HashMap<i32, crate::models::product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();
        // 查询仓库
        let warehouse = warehouse::Entity::find()
            .filter(warehouse::Column::WarehouseCode.eq(&request.warehouse_code))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("仓库不存在"))?;
        // P1 3-7/5-1 修复（批次 62）：保存发货明细快照用于事件发布
        let shipped_items_snapshot: Vec<(i32, rust_decimal::Decimal)> =
            request.items.iter().map(|i| (i.product_id, i.quantity)).collect();
        self.check_inventory(request.order_id, &request.items, txn)
            .await?;
        Ok(ShipOrderContext {
            order,
            order_items,
            product_map,
            warehouse,
            shipped_items_snapshot,
        })
    }

    /// 创建发货单主记录
    async fn create_shipment_delivery(
        &self,
        request: &ShipOrderRequest,
        order: &sales_order::Model,
        warehouse: &warehouse::Model,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<sales_delivery::Model, AppError> {
        // P1 3-8 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
        // 原实现基于时间戳，同秒并发会产生重复单号
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            delivery_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    txn,
                    "DN",
                    sales_delivery::Entity,
                    sales_delivery::Column::DeliveryNo,
                )
                .await?,
            ),
            order_id: Set(request.order_id),
            customer_id: Set(order.customer_id),
            warehouse_id: Set(warehouse.id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set(delivery_status::SHIPPED.to_string()),
            total_quantity: Set(request.items.iter().map(|i| i.quantity).sum()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(request.remarks.clone()),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        Ok(delivery.insert(txn).await?)
    }

    /// 循环处理发货明细：扣减库存 + 生成库存流水 + 累加金额 + 批量 INSERT
    async fn process_shipment_items(
        &self,
        request: &ShipOrderRequest,
        ctx: &ShipOrderContext,
        delivery: &sales_delivery::Model,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ShipmentItemsResult, AppError> {
        let order_item_map: std::collections::HashMap<i32, &sales_order_item::Model> =
            ctx.order_items.iter().map(|oi| (oi.product_id, oi)).collect();
        let mut delivery_items_to_insert: Vec<sales_delivery_item::ActiveModel> = Vec::new();
        let mut pending_inventory_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();
        let mut delivery_total_amount = Decimal::ZERO;
        let mut delivery_total_tax = Decimal::ZERO;
        for item in &request.items {
            let (unit_price, line_amount, line_tax) = Self::compute_line_amounts(&order_item_map, item);
            delivery_items_to_insert.push(Self::build_delivery_item(
                item,
                delivery.id,
                unit_price,
                line_amount,
            ));
            delivery_total_amount += line_amount;
            delivery_total_tax += line_tax;
            let (qty_before, qty_after, stock_color_no, stock_dye_lot_no) = self
                .reduce_inventory(item.product_id, ctx.warehouse.id, item.quantity, request.order_id, txn)
                .await?;
            let quantity_kg = Self::compute_quantity_kg(&ctx.product_map, item.product_id, item.quantity);
            let args = Self::build_record_transaction_args(
                item,
                request,
                ctx,
                qty_before,
                qty_after,
                stock_color_no,
                stock_dye_lot_no,
                quantity_kg,
                user_id,
            );
            let (_, txn_event) =
                crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
                    txn, args,
                )
                .await?;
            if let Some(ev) = txn_event {
                pending_inventory_events.push(ev);
            }
            Self::update_order_item_shipped_qty(txn, request.order_id, item.product_id, item.quantity)
                .await?;
        }
        if !delivery_items_to_insert.is_empty() {
            sales_delivery_item::Entity::insert_many(delivery_items_to_insert)
                .exec(txn)
                .await?;
        }
        Ok(ShipmentItemsResult {
            delivery_total_amount,
            delivery_total_tax,
            pending_inventory_events,
        })
    }

    fn compute_line_amounts(
        order_item_map: &std::collections::HashMap<i32, &sales_order_item::Model>,
        item: &ShipOrderItemRequest,
    ) -> (Decimal, Decimal, Decimal) {
        let (unit_price, tax_percent) = order_item_map
            .get(&item.product_id)
            .map(|oi| (oi.unit_price, oi.tax_percent))
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let line_amount = (item.quantity * unit_price).round_dp(2);
        let line_tax = (line_amount * tax_percent / Decimal::new(100, 0)).round_dp(2);
        (unit_price, line_amount, line_tax)
    }

    fn build_delivery_item(
        item: &ShipOrderItemRequest,
        delivery_id: i32,
        unit_price: Decimal,
        line_amount: Decimal,
    ) -> sales_delivery_item::ActiveModel {
        sales_delivery_item::ActiveModel {
            id: Default::default(),
            delivery_id: Set(delivery_id),
            product_id: Set(item.product_id),
            quantity: Set(item.quantity),
            batch_no: Set(item.batch_no.clone()),
            color_no: Set(item.color_no.clone()),
            dye_lot_id: Set(None),
            dye_lot_no: Set(item.dye_lot_no.clone()),
            remarks: Set(None),
            unit_price: Set(unit_price),
            amount: Set(line_amount),
            created_at: Set(chrono::Utc::now()),
        }
    }

    fn compute_quantity_kg(
        product_map: &std::collections::HashMap<i32, crate::models::product::Model>,
        product_id: i32,
        quantity: Decimal,
    ) -> Decimal {
        product_map
            .get(&product_id)
            .and_then(|p| {
                let gram_weight = p.gram_weight?;
                let width = p.width?;
                crate::utils::dual_unit_converter::DualUnitConverter::meters_to_kg(
                    quantity,
                    gram_weight,
                    width,
                )
                .ok()
            })
            .unwrap_or(Decimal::ZERO)
    }

    fn build_record_transaction_args(
        item: &ShipOrderItemRequest,
        request: &ShipOrderRequest,
        ctx: &ShipOrderContext,
        qty_before: Decimal,
        qty_after: Decimal,
        stock_color_no: String,
        stock_dye_lot_no: Option<String>,
        quantity_kg: Decimal,
        user_id: i32,
    ) -> crate::services::inventory_stock_query::RecordTransactionArgs {
        crate::services::inventory_stock_query::RecordTransactionArgs {
            transaction_type: "SALES_DELIVERY".to_string(),
            product_id: item.product_id,
            warehouse_id: ctx.warehouse.id,
            batch_no: item.batch_no.clone().unwrap_or_default(),
            color_no: stock_color_no,
            dye_lot_no: stock_dye_lot_no,
            grade: String::new(),
            quantity_meters: item.quantity,
            quantity_kg,
            source_bill_type: Some("sales_order".to_string()),
            source_bill_no: Some(ctx.order.order_no.clone()),
            source_bill_id: Some(request.order_id),
            quantity_before_meters: Some(qty_before),
            quantity_before_kg: None,
            quantity_after_meters: Some(qty_after),
            quantity_after_kg: None,
            notes: Some(format!("销售出库 - 订单 {}", ctx.order.order_no)),
            created_by: Some(user_id),
        }
    }

    async fn update_order_item_shipped_qty(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
        product_id: i32,
        quantity: Decimal,
    ) -> Result<(), AppError> {
        sales_order_item::Entity::update_many()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .filter(sales_order_item::Column::ProductId.eq(product_id))
            .col_expr(
                sales_order_item::Column::ShippedQuantity,
                sea_orm::sea_query::Expr::col(sales_order_item::Column::ShippedQuantity)
                    .add(quantity),
            )
            .col_expr(
                sales_order_item::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;
        Ok(())
    }

    /// 更新订单状态 + 全额发货时生成 AR 应收
    async fn update_order_after_shipment(
        &self,
        request: &ShipOrderRequest,
        ctx: ShipOrderContext,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ShipPostCommitContext, AppError> {
        // 更新订单状态
        let order_items_total: Vec<sales_order_item::Model> = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(txn)
            .await?;
        // D12 重构：全额发货判断提取到 check_order_fully_shipped（消除 for + if 分支）
        let is_fully_shipped = Self::check_order_fully_shipped(&order_items_total);
        let new_status = if is_fully_shipped {
            so_status::SHIPPED
        } else {
            so_status::PARTIAL_SHIPPED
        };
        // P1 3-7/5-1 修复（批次 62）：保存发货上下文用于 AR 生成和事件发布
        // order 在下方 .into() 被消费，提前保存所需字段
        // F-P0-6 修复（批次 382 v13 复审）：移除 ship_order_no，收入凭证改用 delivery_no
        let ship_customer_id = ctx.order.customer_id;
        let ship_order_total = ctx.order.total_amount;
        let ship_order_id = request.order_id;
        let ship_items_for_event: Vec<crate::services::event_bus::ShippedItem> = ctx
            .shipped_items_snapshot
            .iter()
            .map(|(pid, qty)| crate::services::event_bus::ShippedItem {
                product_id: *pid,
                quantity: *qty,
            })
            .collect();
        let mut order_update: sales_order::ActiveModel = ctx.order.into();
        order_update.status = Set(new_status.to_string());
        order_update.ship_date = Set(Some(chrono::Utc::now()));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_update,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;
        // P1 3-7/5-1 修复（批次 62）：销售→AR 业务流补全
        // 修复：全额发货时在 commit 前调用 create_receivable 生成 AR（与订单状态更新共用事务）
        if is_fully_shipped {
            // 查询客户账期（payment_terms <= 0 时 create_receivable 内部回退 30 天）
            let customer = crate::models::customer::Entity::find_by_id(ship_customer_id)
                .one(txn)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("客户 {} 不存在", ship_customer_id))
                })?;
            let payment_terms = customer.payment_terms;
            let ar_service =
                crate::services::ar::ArReconciliationService::new(self.db.clone());
            ar_service
                .create_receivable(
                    ship_customer_id,
                    ship_order_id,
                    ship_order_total,
                    payment_terms,
                    user_id,
                    txn,
                )
                .await?;
        }
        Ok(ShipPostCommitContext {
            ship_customer_id,
            ship_order_id,
            ship_items_for_event,
        })
    }

    /// 提交事务后：收入凭证生成 + 库存流水事件 + 销售发货事件发布
    async fn post_commit_shipment_effects(
        &self,
        delivery: &sales_delivery::Model,
        post_ctx: ShipPostCommitContext,
        items_result: ShipmentItemsResult,
        user_id: i32,
    ) {
        // D12 重构：收入凭证生成提取到 create_revenue_voucher_for_delivery（消除 if + if let Err 分支）
        // F-P0-3+F-P0-6 修复（批次 381+382 v13 复审）：每次发货都生成收入确认凭证
        // 借：应收账款（含税总额，挂客户辅助核算）
        // 贷：主营业务收入（不含税）/ 应交税费-销项税额
        // 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
        self.create_revenue_voucher_for_delivery(
            delivery,
            items_result.delivery_total_amount,
            items_result.delivery_total_tax,
            post_ctx.ship_customer_id,
            user_id,
        )
        .await;
        // 批次 356 v13 复审 B-P0-2 修复：commit 后统一发布库存流水事件
        // 触发 inventory_finance_bridge_service 自动生成销售出库凭证
        for ev in items_result.pending_inventory_events {
            crate::services::event_bus::EVENT_BUS.publish(ev);
        }
        // P1 5-1 修复（批次 62）：commit 后发布 SalesOrderShipped 事件
        // 事件发布必须在 commit 之后，避免消费者读到未提交数据。
        // 监听器（event_bus.rs）消费此事件触发财务指标刷新（5-2 修复）。
        crate::services::event_bus::EVENT_BUS
            .publish(crate::services::event_bus::BusinessEvent::SalesOrderShipped {
                order_id: post_ctx.ship_order_id,
                customer_id: post_ctx.ship_customer_id,
                items: post_ctx.ship_items_for_event,
            });
    }

    /// 判断订单是否全额发货（所有明细 shipped_quantity >= quantity）
    fn check_order_fully_shipped(order_items_total: &[sales_order_item::Model]) -> bool {
        for oi in order_items_total {
            if oi.shipped_quantity < oi.quantity {
                return false;
            }
        }
        true
    }

    /// 为发货单生成收入确认凭证（借应收/贷收入+销项税）
    /// 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
    async fn create_revenue_voucher_for_delivery(
        &self,
        delivery: &sales_delivery::Model,
        delivery_total_amount: Decimal,
        delivery_total_tax: Decimal,
        ship_customer_id: i32,
        user_id: i32,
    ) {
        let delivery_total_incl_tax = delivery_total_amount + delivery_total_tax;
        if delivery_total_incl_tax <= Decimal::ZERO {
            return;
        }
        let voucher_req = Self::build_revenue_voucher_request(
            delivery,
            delivery_total_amount,
            delivery_total_tax,
            delivery_total_incl_tax,
            ship_customer_id,
        );
        let voucher_service =
            crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "发货单 {} 收入凭证生成失败：{}",
                delivery.delivery_no,
                e
            );
        }
    }

    /// 构建销售出库收入确认凭证请求（借应收/贷收入/贷销项税 三行分录）
    fn build_revenue_voucher_request(
        delivery: &sales_delivery::Model,
        delivery_total_amount: Decimal,
        delivery_total_tax: Decimal,
        delivery_total_incl_tax: Decimal,
        ship_customer_id: i32,
    ) -> crate::services::voucher_service::CreateVoucherRequest {
        let summary = format!("销售出库收入确认-{}", delivery.delivery_no);
        crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: chrono::Utc::now().date_naive(),
            source_type: Some("SALES_DELIVERY".to_string()),
            source_module: Some("sales".to_string()),
            source_bill_id: Some(delivery.id),
            source_bill_no: Some(delivery.delivery_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                Self::build_revenue_voucher_item(
                    1,
                    "1131",
                    "应收账款",
                    delivery_total_incl_tax,
                    Decimal::ZERO,
                    &summary,
                    ship_customer_id,
                ),
                Self::build_revenue_voucher_item(
                    2,
                    "6001",
                    "主营业务收入",
                    Decimal::ZERO,
                    delivery_total_amount,
                    &summary,
                    ship_customer_id,
                ),
                Self::build_revenue_voucher_item(
                    3,
                    "222101",
                    "应交税费-应交增值税-销项税额",
                    Decimal::ZERO,
                    delivery_total_tax,
                    &summary,
                    ship_customer_id,
                ),
            ],
        }
    }

    /// 构建收入确认凭证的单行分录（客户辅助核算项固定，其余辅助核算项为 None）
    fn build_revenue_voucher_item(
        line_no: i32,
        subject_code: &str,
        subject_name: &str,
        debit: Decimal,
        credit: Decimal,
        summary: &str,
        ship_customer_id: i32,
    ) -> crate::services::voucher_service::VoucherItemRequest {
        crate::services::voucher_service::VoucherItemRequest {
            line_no: Some(line_no),
            subject_code: Some(subject_code.to_string()),
            subject_name: Some(subject_name.to_string()),
            debit,
            credit,
            summary: Some(summary.to_string()),
            assist_customer_id: Some(ship_customer_id),
            assist_supplier_id: None,
            assist_department_id: None,
            assist_employee_id: None,
            assist_project_id: None,
            assist_batch_id: None,
            assist_color_no_id: None,
            assist_dye_lot_id: None,
            assist_grade: None,
            assist_workshop_id: None,
            quantity_meters: None,
            quantity_kg: None,
            unit_price: None,
        }
    }
}
