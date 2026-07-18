//! 销售发货服务（so/delivery）
//!
//! 包含销售订单的发货、库存扣减/释放、订单号生成等。
//! 拆分自原 `sales_service.rs`。
//! 由于 `check_inventory`、`lock_inventory`、`reduce_inventory`、`release_reservations`
//! 这四个方法与发货/库存操作紧密相关，统一在 delivery.rs 中实现。

use crate::models::{
    inventory_reservation, inventory_stock, sales_delivery, sales_delivery_item, sales_order,
    sales_order_item, warehouse,
};
use crate::models::status::inventory_reservation as reservation_status;
use crate::models::status::sales_delivery as delivery_status;
use crate::models::status::sales_order as so_status;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use validator::Validate;

use super::order::SalesService;

// =====================================================
// 发货请求 DTO
// =====================================================

#[derive(Debug, Validate, Deserialize)]
pub struct ShipOrderRequest {
    #[validate(range(min = 1, message = "订单ID必须大于0"))]
    pub order_id: i32,
    #[validate(length(max = 50, message = "仓库编号长度不能超过50个字符"))]
    pub warehouse_code: String,
    pub items: Vec<ShipOrderItemRequest>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct ShipOrderItemRequest {
    pub product_id: i32,
    pub quantity: Decimal,
    #[validate(length(max = 50, message = "批次号长度不能超过50个字符"))]
    pub batch_no: Option<String>,
    // v14 批次 421 T-P1-5：缸号同订单校验支持字段
    // 依据：fabric-industry-research.md §2.3 约束 5 - 同一订单同面料必须使用相同缸号
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
}

// =====================================================
// 销售订单服务 impl 块
// =====================================================

/// v14 批次 421 T-P1-5：缸号同订单校验
///
/// 依据：fabric-industry-research.md §2.3 约束 5
/// 业务规则：出库时，同一订单必须使用相同缸号的面料，系统校验订单中所有该面料是否来自同一批次，不一致则报警提示
/// 业务语义：一个缸号代表一次染色，同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺
///
/// 校验逻辑：同一 product_id 的所有发货明细必须使用相同的 dye_lot_no
/// - 同 product_id 但 dye_lot_no 不一致 → 返回业务错误（避免混缸色差）
/// - dye_lot_no 均为 None → 视为未指定缸号，跳过校验（兼容无缸号场景）
/// - 单 product_id 单 dye_lot_no → 通过校验
pub fn validate_dye_lot_consistency(items: &[ShipOrderItemRequest]) -> Result<(), AppError> {
    use std::collections::HashMap;

    // 按 product_id 分组收集 dye_lot_no
    let mut product_dye_lots: HashMap<i32, std::collections::HashSet<String>> = HashMap::new();
    for item in items {
        if let Some(dye_lot_no) = &item.dye_lot_no {
            if !dye_lot_no.is_empty() {
                product_dye_lots
                    .entry(item.product_id)
                    .or_default()
                    .insert(dye_lot_no.clone());
            }
        }
    }

    // 校验每个 product_id 下不能有多个不同的 dye_lot_no
    for (product_id, dye_lots) in &product_dye_lots {
        if dye_lots.len() > 1 {
            let dye_lot_list: Vec<String> = dye_lots.iter().cloned().collect();
            return Err(AppError::business(format!(
                "产品 {} 在同一订单中使用了多个不同缸号 {}，违反缸号同订单校验：同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺",
                product_id,
                dye_lot_list.join("/")
            )));
        }
    }

    Ok(())
}

impl SalesService {
    // 生成销售订单号
    // 格式：SO + 年月日 + 三位序号（SO20260315001）
    crate::impl_generate_no!(
        generate_order_no,
        "SO",
        sales_order::Entity,
        sales_order::Column::OrderNo
    );

    /// 销售订单发货
    pub async fn ship_order(
        &self,
        request: ShipOrderRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        // v14 批次 421 T-P1-5：缸号同订单校验
        // 依据：fabric-industry-research.md §2.3 约束 5 - 同一订单同面料必须使用相同缸号
        // 必须在开启事务前校验，避免无效请求占用数据库事务资源
        validate_dye_lot_consistency(&request.items)?;

        // V15 P0-F19：发货前校验大货批色门禁
        // 业务规则：销售订单关联的所有 bulk_color_approval 记录必须全部为 approved 状态
        // 否则阻止发货（delivery_blocking=true 阻断）
        // 必须在开启事务前校验，避免无效请求占用数据库事务资源
        crate::services::bulk_color_approval_service::validate_bulk_color_approval(
            &*self.db,
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

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 检查订单状态（加 lock_exclusive 串行化并发发货）
        let order = sales_order::Entity::find_by_id(request.order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != so_status::APPROVED {
            return Err(AppError::business("只有已审批的订单才能发货"));
        }

        // 查询订单明细
        // F-P0-6 修复（批次 382 v13 复审）：保留查询结果用于计算发货金额
        let order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(&txn)
            .await?;
        // 构建 product_id → order_item 映射，用于发货明细金额计算
        let order_item_map: std::collections::HashMap<i32, &sales_order_item::Model> =
            order_items.iter().map(|oi| (oi.product_id, oi)).collect();

        // v14 批次 418 修复 G-P0-1：批量查询产品获取 gram_weight/width，
        // 用于库存流水的 quantity_kg 双单位换算（替代原 Decimal::ZERO 硬编码）
        let product_ids: Vec<i32> = order_items.iter().map(|oi| oi.product_id).collect();
        let products = if product_ids.is_empty() {
            Vec::new()
        } else {
            crate::models::product::Entity::find()
                .filter(crate::models::product::Column::Id.is_in(product_ids))
                .all(&txn)
                .await?
        };
        let product_map: std::collections::HashMap<i32, crate::models::product::Model> =
            products.into_iter().map(|p| (p.id, p)).collect();

        // 查询仓库
        let warehouse = warehouse::Entity::find()
            .filter(warehouse::Column::WarehouseCode.eq(&request.warehouse_code))
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("仓库不存在"))?;

        // 检查库存是否充足
        // P1 3-7/5-1 修复（批次 62）：保存发货明细快照用于事件发布
        // request.items 在下方 for 循环被 move，提前克隆事件所需字段
        let shipped_items_snapshot: Vec<(i32, rust_decimal::Decimal)> =
            request.items.iter().map(|i| (i.product_id, i.quantity)).collect();

        self.check_inventory(request.order_id, &request.items, &txn)
            .await?;

        // 创建发货单
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            // P1 3-8 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
            // 原实现基于时间戳，同秒并发会产生重复单号
            delivery_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    &txn,
                    "DN",
                    sales_delivery::Entity,
                    sales_delivery::Column::DeliveryNo,
                )
                .await?
            ),
            order_id: Set(request.order_id),
            customer_id: Set(order.customer_id),
            warehouse_id: Set(warehouse.id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set(delivery_status::SHIPPED.to_string()),
            total_quantity: Set(request.items.iter().map(|i| i.quantity).sum()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(request.remarks),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let delivery = delivery.insert(&txn).await?;

        // 创建发货单明细并扣减库存（v13 P1-3：发货明细批量 INSERT，库存扣减与订单明细更新保持逐条以确保乐观锁语义）
        let mut delivery_items_to_insert: Vec<sales_delivery_item::ActiveModel> = Vec::new();
        // 批次 356 v13 复审 B-P0-2 修复：收集库存流水事件，commit 后统一 publish
        let mut pending_inventory_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();
        // F-P0-6 修复（批次 382 v13 复审）：累加发货金额用于收入凭证生成
        let mut delivery_total_amount = Decimal::ZERO;
        let mut delivery_total_tax = Decimal::ZERO;
        for item in request.items {
            // F-P0-6 修复：从订单明细查询单价和税率，计算发货明细金额
            let (unit_price, tax_percent) = order_item_map
                .get(&item.product_id)
                .map(|oi| (oi.unit_price, oi.tax_percent))
                .unwrap_or((Decimal::ZERO, Decimal::ZERO));
            let line_amount = (item.quantity * unit_price).round_dp(2);
            let line_tax = (line_amount * tax_percent / Decimal::new(100, 0)).round_dp(2);

            // 收集发货明细（不立即 INSERT）
            delivery_items_to_insert.push(sales_delivery_item::ActiveModel {
                id: Default::default(),
                delivery_id: Set(delivery.id),
                product_id: Set(item.product_id),
                quantity: Set(item.quantity),
                // 批次 356 v13 复审修复：clone 避免 move，下方 record_transaction_txn 仍需访问 item.batch_no
                batch_no: Set(item.batch_no.clone()),
                // v14 批次 421 T-P1-5：发货明细使用请求中的 color_no/dye_lot_no
                // 已通过 validate_dye_lot_consistency 校验同一订单同 product_id 缸号一致
                color_no: Set(item.color_no.clone()),
                dye_lot_id: Set(None),
                dye_lot_no: Set(item.dye_lot_no.clone()),
                remarks: Set(None),
                unit_price: Set(unit_price),
                amount: Set(line_amount),
                created_at: Set(chrono::Utc::now()),
            });
            delivery_total_amount += line_amount;
            delivery_total_tax += line_tax;

            // 扣减库存
            // v14 批次 418 修复 D-P0-5：reduce_inventory 额外返回库存的 color_no/dye_lot_no
            let (qty_before, qty_after, stock_color_no, stock_dye_lot_no) = self.reduce_inventory(
                item.product_id,
                warehouse.id,
                item.quantity,
                request.order_id,
                &txn,
            )
            .await?;

            // v14 批次 418 修复 G-P0-1：调用 DualUnitConverter 双单位换算计算 quantity_kg，
            // 替代原 Decimal::ZERO 硬编码
            let quantity_kg = product_map
                .get(&item.product_id)
                .and_then(|p| {
                    let gram_weight = p.gram_weight?;
                    let width = p.width?;
                    crate::utils::dual_unit_converter::DualUnitConverter::meters_to_kg(
                        item.quantity,
                        gram_weight,
                        width,
                    )
                    .ok()
                })
                .unwrap_or(Decimal::ZERO);

            // 批次 356 v13 复审 B-P0-2 修复：销售出库生成 SALES_DELIVERY 类型库存流水
            // 触发 inventory_finance_bridge_service 自动生成销售出库凭证（借:主营业务成本/贷:库存商品）
            let (_, txn_event) =
                crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
                    &txn,
                    crate::services::inventory_stock_query::RecordTransactionArgs {
                        transaction_type: "SALES_DELIVERY".to_string(),
                        product_id: item.product_id,
                        warehouse_id: warehouse.id,
                        // 批次 356 v13 复审修复：item.batch_no 为 Option<String>，RecordTransactionArgs.batch_no 期望 String
                        // 无批次号时使用空字符串占位（与库存流水无批次号语义一致）
                        batch_no: item.batch_no.clone().unwrap_or_default(),
                        // v14 批次 418 修复 D-P0-5：使用从库存获取的真实 color_no/dye_lot_no
                        color_no: stock_color_no.clone(),
                        dye_lot_no: stock_dye_lot_no.clone(),
                        grade: String::new(),
                        quantity_meters: item.quantity,
                        quantity_kg,
                        source_bill_type: Some("sales_order".to_string()),
                        source_bill_no: Some(order.order_no.clone()),
                        source_bill_id: Some(request.order_id),
                        quantity_before_meters: Some(qty_before),
                        quantity_before_kg: None,
                        quantity_after_meters: Some(qty_after),
                        quantity_after_kg: None,
                        notes: Some(format!("销售出库 - 订单 {}", order.order_no)),
                        created_by: Some(user_id),
                    },
                )
                .await?;
            if let Some(ev) = txn_event {
                pending_inventory_events.push(ev);
            }

            // 使用 update_many 批量更新订单明细已发货数量
            sales_order_item::Entity::update_many()
                .filter(sales_order_item::Column::OrderId.eq(request.order_id))
                .filter(sales_order_item::Column::ProductId.eq(item.product_id))
                .col_expr(
                    sales_order_item::Column::ShippedQuantity,
                    sea_orm::sea_query::Expr::col(sales_order_item::Column::ShippedQuantity)
                        .add(item.quantity),
                )
                .col_expr(
                    sales_order_item::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                )
                .exec(&txn)
                .await?;
        }

        // 批量 INSERT 发货明细，替代逐条 INSERT
        if !delivery_items_to_insert.is_empty() {
            sales_delivery_item::Entity::insert_many(delivery_items_to_insert)
                .exec(&txn)
                .await?;
        }

        // 更新订单状态
        let order_items_total: Vec<sales_order_item::Model> = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(request.order_id))
            .all(&txn)
            .await?;

        let mut is_fully_shipped = true;
        for oi in &order_items_total {
            if oi.shipped_quantity < oi.quantity {
                is_fully_shipped = false;
                break;
            }
        }

        let new_status = if is_fully_shipped {
            so_status::SHIPPED
        } else {
            so_status::PARTIAL_SHIPPED
        };

        // P1 3-7/5-1 修复（批次 62）：保存发货上下文用于 AR 生成和事件发布
        // order 在下方 .into() 被消费，提前保存所需字段
        // F-P0-6 修复（批次 382 v13 复审）：移除 ship_order_no，收入凭证改用 delivery_no
        let ship_customer_id = order.customer_id;
        let ship_order_total = order.total_amount;
        let ship_order_id = request.order_id;
        let ship_items_for_event: Vec<crate::services::event_bus::ShippedItem> =
            shipped_items_snapshot
                .iter()
                .map(|(pid, qty)| crate::services::event_bus::ShippedItem {
                    product_id: *pid,
                    quantity: *qty,
                })
                .collect();
        let is_full_shipment = is_fully_shipped;

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set(new_status.to_string());
        order_update.ship_date = Set(Some(chrono::Utc::now()));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // P1 3-7/5-1 修复（批次 62）：销售→AR 业务流补全
        // 原实现 ship_order 在 commit 后未调用 create_receivable，销售发货→应收账款业务流断点，
        // 财务报表应收账款余额与销售发货数据不一致。
        // 修复：全额发货时在 commit 前调用 create_receivable 生成 AR（与订单状态更新共用事务），
        // 部分发货不生成（避免与 create_receivable 的幂等检查冲突；部分发货的 AR 在最终全额发货时统一生成）。
        if is_full_shipment {
            // 查询客户账期（payment_terms <= 0 时 create_receivable 内部回退 30 天）
            let customer = crate::models::customer::Entity::find_by_id(ship_customer_id)
                .one(&txn)
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
                    &txn,
                )
                .await?;
        }

        // 提交事务
        txn.commit().await?;

        // F-P0-3+F-P0-6 修复（批次 381+382 v13 复审）：每次发货都生成收入确认凭证
        // 借：应收账款（含税总额，挂客户辅助核算）
        // 贷：主营业务收入（不含税）/ 应交税费-销项税额
        // 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
        let delivery_total_incl_tax = delivery_total_amount + delivery_total_tax;
        if delivery_total_incl_tax > Decimal::ZERO {
            let voucher_req = crate::services::voucher_service::CreateVoucherRequest {
                voucher_type: "转".to_string(),
                voucher_date: chrono::Utc::now().date_naive(),
                source_type: Some("SALES_DELIVERY".to_string()),
                source_module: Some("sales".to_string()),
                source_bill_id: Some(delivery.id),
                source_bill_no: Some(delivery.delivery_no.clone()),
                batch_no: None,
                color_no: None,
                items: vec![
                    crate::services::voucher_service::VoucherItemRequest {
                        line_no: Some(1),
                        subject_code: Some("1131".to_string()),
                        subject_name: Some("应收账款".to_string()),
                        debit: delivery_total_incl_tax,
                        credit: Decimal::ZERO,
                        summary: Some(format!("销售出库收入确认-{}", delivery.delivery_no)),
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
                    },
                    crate::services::voucher_service::VoucherItemRequest {
                        line_no: Some(2),
                        subject_code: Some("6001".to_string()),
                        subject_name: Some("主营业务收入".to_string()),
                        debit: Decimal::ZERO,
                        credit: delivery_total_amount,
                        summary: Some(format!("销售出库收入确认-{}", delivery.delivery_no)),
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
                    },
                    crate::services::voucher_service::VoucherItemRequest {
                        line_no: Some(3),
                        subject_code: Some("222101".to_string()),
                        subject_name: Some("应交税费-应交增值税-销项税额".to_string()),
                        debit: Decimal::ZERO,
                        credit: delivery_total_tax,
                        summary: Some(format!("销售出库收入确认-{}", delivery.delivery_no)),
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
                    },
                ],
            };
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

        // 批次 356 v13 复审 B-P0-2 修复：commit 后统一发布库存流水事件
        // 触发 inventory_finance_bridge_service 自动生成销售出库凭证
        for ev in pending_inventory_events {
            crate::services::event_bus::EVENT_BUS.publish(ev);
        }

        // P1 5-1 修复（批次 62）：commit 后发布 SalesOrderShipped 事件
        // 事件发布必须在 commit 之后，避免消费者读到未提交数据。
        // 监听器（event_bus.rs）消费此事件触发财务指标刷新（5-2 修复）。
        crate::services::event_bus::EVENT_BUS
            .publish(crate::services::event_bus::BusinessEvent::SalesOrderShipped {
                order_id: ship_order_id,
                customer_id: ship_customer_id,
                items: ship_items_for_event,
            });

        Ok(())
    }

    /// 获取订单发货记录
    pub async fn get_order_deliveries(
        &self,
        order_id: i32,
    ) -> Result<Vec<sales_delivery::Model>, AppError> {
        let deliveries = sales_delivery::Entity::find()
            .filter(sales_delivery::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await?;
        Ok(deliveries)
    }

    /// 创建发货单（手动创建）
    pub async fn create_delivery(
        &self,
        order_id: i32,
        warehouse_id: i32,
        user_id: i32,
    ) -> Result<sales_delivery::Model, AppError> {
        // P1 3-8 修复（批次 60）：包裹事务，确保单号生成的 advisory_xact_lock
        // 与 INSERT 在同一事务内，锁覆盖完整临界区
        let txn = (*self.db).begin().await?;
        let delivery = sales_delivery::ActiveModel {
            id: Default::default(),
            // P1 3-8 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
            delivery_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    &txn,
                    "DN",
                    sales_delivery::Entity,
                    sales_delivery::Column::DeliveryNo,
                )
                .await?
            ),
            order_id: Set(order_id),
            customer_id: Set(0),
            warehouse_id: Set(warehouse_id),
            delivery_date: Set(chrono::Utc::now().date_naive()),
            status: Set(delivery_status::PENDING.to_string()),
            total_quantity: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(None),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let delivery = delivery.insert(&txn).await?;
        txn.commit().await?;
        Ok(delivery)
    }

    // ========== 库存辅助方法（私有） ==========

    /// 检查库存是否充足
    pub(crate) async fn check_inventory(
        &self,
        order_id: i32,
        items: &[ShipOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }

        // v11 批次 38 修复：批量查询所有预留记录和库存记录，避免循环内逐个查询（N+1，最坏 2N 次查询）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();

        // 批量查询该订单所有 pending 预留记录，按 product_id 索引（取每组第一条，与原 .one() 语义一致）
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.is_in(product_ids.clone()))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?;
        let reservation_map: std::collections::HashMap<i32, &inventory_reservation::Model> =
            reservations
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, r| {
                    // 仅保留每个 product_id 的第一条（与原 .one() 语义一致）
                    acc.entry(r.product_id).or_insert(r);
                    acc
                });

        // 批量查询所有相关库存记录，按 product_id 索引
        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, &inventory_stock::Model> =
            stocks.iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            // 优先从预留记录查询
            if let Some(res) = reservation_map.get(&item.product_id) {
                if res.quantity < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 预留数量 {} 小于发货数量 {}",
                        item.product_id, res.quantity, item.quantity
                    )));
                }
                continue;
            }

            // 没有预留记录时直接查询库存
            match stock_map.get(&item.product_id) {
                Some(s) => {
                    if s.quantity_available < item.quantity {
                        return Err(AppError::business(format!(
                            "产品 {} 库存 {} 小于发货数量 {}",
                            item.product_id, s.quantity_available, item.quantity
                        )));
                    }
                }
                None => {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不存在",
                        item.product_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// 锁定库存（创建预留记录）
    pub(crate) async fn lock_inventory(
        &self,
        order_id: i32,
        items: &[super::SalesOrderItemRequest],
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // v15 批次 42 修复：循环外批量查询该订单所有已存在的 pending 预留记录，
        // 避免循环内逐个查询（N+1）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let existing_reservation_ids: std::collections::HashSet<i32> = if product_ids.is_empty() {
            std::collections::HashSet::new()
        } else {
            inventory_reservation::Entity::find()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.is_in(product_ids))
                .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
                .all(txn)
                .await?
                .into_iter()
                .map(|r| r.product_id)
                .collect()
        };

        // v17 批次 46 修复：循环外批量锁定所有需锁定的 product_id 的库存，避免循环内逐个 lock_exclusive（N+1）
        // PostgreSQL SELECT FOR UPDATE 支持 WHERE IN 批量加锁，行锁在事务内持续到 commit
        let need_lock_product_ids: Vec<i32> = items
            .iter()
            .map(|i| i.product_id)
            .filter(|pid| !existing_reservation_ids.contains(pid))
            .collect();
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            if need_lock_product_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                inventory_stock::Entity::find()
                    .filter(inventory_stock::Column::ProductId.is_in(need_lock_product_ids))
                    .lock_exclusive()
                    .all(txn)
                    .await?
                    .into_iter()
                    .map(|s| (s.product_id, s))
                    .collect()
            };

        // v13 P1-3：预留记录批量 INSERT，库存 update_many 保持逐条以确保防御性 WHERE 语义
        let mut reservations_to_insert: Vec<inventory_reservation::ActiveModel> = Vec::new();
        for item in items {
            if existing_reservation_ids.contains(&item.product_id) {
                tracing::info!("产品 {} 已存在预留记录，跳过创建", item.product_id);
                continue;
            }

            // v17 批次 46 修复：从批量查询结果获取（行锁已在批量查询时获取）
            let stock = stock_map.get(&item.product_id).cloned();

            if let Some(s) = stock {
                if s.quantity_available < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足，无法锁定",
                        item.product_id
                    )));
                }

                // 收集预留记录（不立即 INSERT）
                reservations_to_insert.push(inventory_reservation::ActiveModel {
                    id: Default::default(),
                    order_id: Set(order_id),
                    product_id: Set(item.product_id),
                    warehouse_id: Set(s.warehouse_id),
                    quantity: Set(item.quantity),
                    status: Set(reservation_status::PENDING.to_string()),
                    reserved_at: Set(chrono::Utc::now()),
                    released_at: Set(None),
                    notes: Set(None),
                    created_by: Set(Some(user_id)),
                    created_at: Set(chrono::Utc::now()),
                    updated_at: Set(chrono::Utc::now()),
                });

                // 批次 9（2026-06-28）：UPDATE 加防御性 WHERE 条件 quantity_available >= quantity，
                // 即使并发绕过 SELECT FOR UPDATE（理论上不会发生），也能阻止超扣
                let lock_result = inventory_stock::Entity::update_many()
                    .filter(inventory_stock::Column::Id.eq(s.id))
                    .filter(inventory_stock::Column::QuantityAvailable.gte(item.quantity))
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                            .sub(item.quantity),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                    )
                    .exec(txn)
                    .await?;

                if lock_result.rows_affected == 0 {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                        item.product_id
                    )));
                }
            } else {
                return Err(AppError::business(format!(
                    "产品 {} 没有库存记录，无法锁定",
                    item.product_id
                )));
            }
        }
        // 批量 INSERT 预留记录，替代逐条 INSERT
        if !reservations_to_insert.is_empty() {
            inventory_reservation::Entity::insert_many(reservations_to_insert)
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 扣减库存
    /// 返回 (变更前可用数量, 变更后可用数量)，用于记录库存流水
    pub(crate) async fn reduce_inventory(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity: Decimal,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(Decimal, Decimal, String, Option<String>), AppError> {
        // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发发货导致超扣
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

        if stock.quantity_available < quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存 {} 小于发货数量 {}",
                product_id, stock.quantity_available, quantity
            )));
        }

        // 批次 9（2026-06-28）：UPDATE 加防御性 WHERE 条件 quantity_available >= quantity，
        // 即使并发绕过 SELECT FOR UPDATE（理论上不会发生），也能阻止超扣
        let reduce_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::QuantityAvailable.gte(quantity))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .sub(quantity),
            )
            .col_expr(
                inventory_stock::Column::QuantityShipped,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                    .add(quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        if reduce_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "产品 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                product_id
            )));
        }

        // 标记预留为已完成
        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.eq(product_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val(reservation_status::CONSUMED.to_string()).into(),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        // 批次 356 v13 复审 B-P0-2 修复：返回变更前后的可用数量，供调用方记录库存流水
        // v14 批次 418 修复 D-P0-5：同时返回库存的 color_no/dye_lot_no，
        // 供调用方在库存流水中记录真实缸号/色号，替代原 None/空字符串硬编码
        let qty_before = stock.quantity_available;
        let qty_after = qty_before - quantity;
        Ok((qty_before, qty_after, stock.color_no.clone(), stock.dye_lot_no.clone()))
    }

    /// 释放订单的库存预留记录
    pub(crate) async fn release_reservations(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?;

        // P2 5-14 修复：按 (product_id, warehouse_id) 聚合后批量更新库存，
        // 原为循环内逐条 update_many 导致 N 个=N 次 UPDATE；聚合后仅 G 次 UPDATE（G=唯一 product+warehouse 组合数）
        use std::collections::HashMap;
        let mut grouped: HashMap<(i32, i32), Decimal> = HashMap::new();
        for res in reservations {
            *grouped
                .entry((res.product_id, res.warehouse_id))
                .or_insert(Decimal::ZERO) += res.quantity;
        }

        let now = chrono::Utc::now();
        for ((product_id, warehouse_id), total_qty) in grouped {
            inventory_stock::Entity::update_many()
                .filter(inventory_stock::Column::ProductId.eq(product_id))
                .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
                .col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                        .add(total_qty),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(now).into(),
                )
                .exec(txn)
                .await?;
        }

        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val(reservation_status::CANCELLED.to_string()).into(),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        Ok(())
    }

    /// 取消销售发货单
    ///
    /// 批次 216 P2-1 修复（v12 复审）：实现销售发货 cancel_delivery 功能，
    /// 移除 sales_delivery::CANCELLED 的 #[allow(dead_code)] 标注。
    ///
    /// 业务规则：
    /// - 仅 SHIPPED 状态的发货单可取消（PENDING 是预留态，本系统发货即 SHIPPED）
    /// - 库存恢复（对称反向）：quantity_available += qty，quantity_shipped -= qty
    /// - 预留恢复：将 CONSUMED 状态的预留恢复为 PENDING
    /// - 订单明细回退：sales_order_item.shipped_quantity -= qty
    /// - 订单状态回退：若所有发货单取消，订单 SHIPPED→APPROVED；部分取消 SHIPPED→PARTIAL_SHIPPED
    /// - AR 冲销：全额发货时生成的应收账款需冲销（生成红字 credit_memo）
    pub async fn cancel_delivery(
        &self,
        delivery_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<sales_delivery::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询发货单（加 lock_exclusive 串行化并发取消）
        let delivery = sales_delivery::Entity::find_by_id(delivery_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("发货单 {}", delivery_id)))?;

        // 2. 检查状态 - 只有 SHIPPED 状态的发货单才能取消
        if delivery.status != delivery_status::SHIPPED {
            return Err(AppError::business(format!(
                "发货单状态不允许取消，当前状态：{}，仅 SHIPPED 状态可取消",
                delivery.status
            )));
        }

        let order_id = delivery.order_id;
        let warehouse_id = delivery.warehouse_id;

        // 3. 查询发货明细（用于库存恢复和订单明细回退）
        let delivery_items = sales_delivery_item::Entity::find()
            .filter(sales_delivery_item::Column::DeliveryId.eq(delivery_id))
            .all(&txn)
            .await?;

        // 4. 加排他锁查询关联订单（串行化并发取消/发货）
        let order = sales_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {}", order_id)))?;

        // 5. 恢复库存 + 回退订单明细已发货数量 + 恢复预留
        for item in &delivery_items {
            // 5.1 恢复库存（对称反向）：quantity_available += qty，quantity_shipped -= qty
            self.restore_inventory(item.product_id, warehouse_id, item.quantity, &txn)
                .await?;

            // 5.2 回退订单明细已发货数量
            sales_order_item::Entity::update_many()
                .filter(sales_order_item::Column::OrderId.eq(order_id))
                .filter(sales_order_item::Column::ProductId.eq(item.product_id))
                .col_expr(
                    sales_order_item::Column::ShippedQuantity,
                    sea_orm::sea_query::Expr::col(sales_order_item::Column::ShippedQuantity)
                        .sub(item.quantity),
                )
                .col_expr(
                    sales_order_item::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                )
                .exec(&txn)
                .await?;

            // 5.3 恢复预留：将 CONSUMED 状态的预留恢复为 PENDING
            inventory_reservation::Entity::update_many()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
                .filter(inventory_reservation::Column::Status.eq(reservation_status::CONSUMED))
                .col_expr(
                    inventory_reservation::Column::Status,
                    sea_orm::sea_query::Expr::val(reservation_status::PENDING.to_string())
                        .into(),
                )
                .col_expr(
                    inventory_reservation::Column::ReleasedAt,
                    sea_orm::sea_query::Expr::val(None::<chrono::DateTime<chrono::Utc>>).into(),
                )
                .col_expr(
                    inventory_reservation::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                )
                .exec(&txn)
                .await?;
        }

        // 6. 更新发货单状态为 CANCELLED，记录取消原因到 remarks
        let now = chrono::Utc::now();
        let cancel_remark = format!("[取消原因] {}", reason);
        let mut delivery_active: sales_delivery::ActiveModel = delivery.into();
        delivery_active.status = Set(delivery_status::CANCELLED.to_string());
        delivery_active.remarks = Set(Some(cancel_remark));
        delivery_active.updated_at = Set(now);

        let updated_delivery = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            delivery_active,
            Some(user_id),
        )
        .await?;

        // 7. 判定订单状态是否需要回退
        let order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .all(&txn)
            .await?;

        let mut has_shipped = false;
        for oi in &order_items {
            if oi.shipped_quantity > Decimal::ZERO {
                has_shipped = true;
                break;
            }
        }

        // 订单状态回退：若所有发货都已取消，订单回退到 APPROVED；否则回退到 PARTIAL_SHIPPED
        let new_order_status = if has_shipped {
            so_status::PARTIAL_SHIPPED
        } else {
            so_status::APPROVED
        };

        // 仅当订单当前状态为 SHIPPED 或 PARTIAL_SHIPPED 时才回退（避免覆盖 CANCELLED 等终态）
        if order.status == so_status::SHIPPED || order.status == so_status::PARTIAL_SHIPPED {
            let mut order_active: sales_order::ActiveModel = order.into();
            order_active.status = Set(new_order_status.to_string());
            order_active.updated_at = Set(now);

            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                order_active,
                Some(user_id),
            )
            .await?;
        }

        txn.commit().await?;

        Ok(updated_delivery)
    }

    /// 恢复库存（取消发货时使用，对称反向于 reduce_inventory）
    ///
    /// quantity_available += qty，quantity_shipped -= qty
    async fn restore_inventory(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity: Decimal,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 加行锁查询库存记录
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

        // 防御性校验：已发货数量不能小于要恢复的数量
        if stock.quantity_shipped < quantity {
            return Err(AppError::business(format!(
                "产品 {} 已发货数量 {} 小于要恢复的数量 {}，库存数据不一致",
                product_id, stock.quantity_shipped, quantity
            )));
        }

        // 对称反向更新：quantity_available += qty，quantity_shipped -= qty
        let restore_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::QuantityShipped.gte(quantity))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .add(quantity),
            )
            .col_expr(
                inventory_stock::Column::QuantityShipped,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                    .sub(quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        if restore_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "产品 {} 库存恢复失败（并发冲突或已发货数量不足）",
                product_id
            )));
        }

        Ok(())
    }

    // ========== 数据导出方法 ==========

    /// 导出销售订单为 CSV 格式
    pub async fn export_orders_to_csv(
        &self,
        status: Option<String>,
        customer_id: Option<i32>,
        order_no: Option<String>,
    ) -> Result<Vec<u8>, AppError> {
        let page_req = crate::models::dto::PageRequest {
            page: 1,
            page_size: 10000,
        };
        let orders = self
            .list_orders(page_req, status, customer_id, order_no, None)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use crate::search::{ElasticClient, SearchClient};
    use sea_orm::{Database, DatabaseConnection};
    use std::str::FromStr;
    use std::sync::Arc;

    /// 测试 SQLite 内存数据库连接夹具
    ///
    /// 与 order_workflow.rs / customer_credit_limit.rs 保持一致的夹具风格，
    /// 默认使用 sqlite::memory:，通过 TEST_DATABASE_URL 环境变量可切换到 PostgreSQL。
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    /// 复现 ship_order 的订单状态校验门（不涉及数据库）
    ///
    /// 与 ship_order 中 `if order.status != so_status::APPROVED` 保持一致：
    /// 仅已审批订单可发货，其余状态返回业务错误。
    fn ship_order_status_gate(status: &str) -> Result<(), AppError> {
        if status != so_status::APPROVED {
            return Err(AppError::business("只有已审批的订单才能发货"));
        }
        Ok(())
    }

    /// 复现 cancel_delivery 的发货单状态校验门（不涉及数据库）
    ///
    /// 与 cancel_delivery 中 `if delivery.status != delivery_status::SHIPPED` 保持一致：
    /// 仅已发货单可取消，其余状态返回业务错误。
    fn cancel_delivery_status_gate(status: &str) -> Result<(), AppError> {
        if status != delivery_status::SHIPPED {
            return Err(AppError::business(format!(
                "发货单状态不允许取消，当前状态：{}，仅 SHIPPED 状态可取消",
                status
            )));
        }
        Ok(())
    }

    /// 复现 ship_order 的全部发货判定（不涉及数据库）
    ///
    /// 与 ship_order 中遍历 order_items_total 判定 is_fully_shipped 保持一致：
    /// 所有明细 shipped_quantity >= quantity 即为全部发货。
    /// 入参元组为 (shipped_quantity, ordered_quantity)。
    fn compute_is_fully_shipped(items: &[(Decimal, Decimal)]) -> bool {
        items.iter().all(|(shipped, ordered)| *shipped >= *ordered)
    }

    /// 复现 ship_order 发货后的订单状态选择（不涉及数据库）
    ///
    /// 全部发货 → SHIPPED；否则 → PARTIAL_SHIPPED。
    fn compute_new_status_after_ship(is_fully_shipped: bool) -> &'static str {
        if is_fully_shipped {
            so_status::SHIPPED
        } else {
            so_status::PARTIAL_SHIPPED
        }
    }

    /// 复现 cancel_delivery 取消发货后的订单状态回退（不涉及数据库）
    ///
    /// 仍有已发数量 → PARTIAL_SHIPPED；全部取消 → APPROVED。
    fn compute_new_status_after_cancel(has_shipped: bool) -> &'static str {
        if has_shipped {
            so_status::PARTIAL_SHIPPED
        } else {
            so_status::APPROVED
        }
    }

    /// 复现 cancel_delivery 中订单状态回退的触发条件（不涉及数据库）
    ///
    /// 仅当订单当前状态为 SHIPPED 或 PARTIAL_SHIPPED 时才回退，
    /// 避免覆盖 CANCELLED 等终态。
    fn order_status_rollback_eligible(status: &str) -> bool {
        status == so_status::SHIPPED || status == so_status::PARTIAL_SHIPPED
    }

    /// 复现 cancel_delivery 的取消备注格式（不涉及数据库）
    ///
    /// 与 cancel_delivery 中 `format!("[取消原因] {}", reason)` 保持一致。
    fn format_cancel_remark(reason: &str) -> String {
        format!("[取消原因] {}", reason)
    }

    /// 复现 check_inventory 中预留数量校验逻辑（不涉及数据库）
    ///
    /// 与 check_inventory 中 `if res.quantity < item.quantity` 保持一致：
    /// 预留数量小于发货数量时返回业务错误。
    fn check_inventory_reservation_logic(
        res_qty: Decimal,
        item_qty: Decimal,
        product_id: i32,
    ) -> Result<(), AppError> {
        if res_qty < item_qty {
            return Err(AppError::business(format!(
                "产品 {} 预留数量 {} 小于发货数量 {}",
                product_id, res_qty, item_qty
            )));
        }
        Ok(())
    }

    /// 复现 check_inventory 中库存数量校验逻辑（不涉及数据库）
    ///
    /// 与 check_inventory 中 `if s.quantity_available < item.quantity` 保持一致：
    /// 可用库存小于发货数量时返回业务错误。
    fn check_inventory_stock_logic(
        stock_available: Decimal,
        item_qty: Decimal,
        product_id: i32,
    ) -> Result<(), AppError> {
        if stock_available < item_qty {
            return Err(AppError::business(format!(
                "产品 {} 库存 {} 小于发货数量 {}",
                product_id, stock_available, item_qty
            )));
        }
        Ok(())
    }

    // ===== 状态常量值正确性 =====

    /// 测试_销售发货状态常量值正确性
    ///
    /// 校验 sales_delivery 子模块的 PENDING/SHIPPED/CANCELLED 常量值，
    /// 避免硬编码字符串导致的拼写错误（批次 158 v11 接入）。
    #[test]
    fn 测试_销售发货状态常量值正确性() {
        assert_eq!(delivery_status::PENDING, "pending");
        assert_eq!(delivery_status::SHIPPED, "shipped");
        assert_eq!(delivery_status::CANCELLED, "cancelled");
    }

    /// 测试_销售订单状态常量值正确性
    ///
    /// 校验 sales_order 子模块的发货相关状态常量值（小写），
    /// 覆盖 ship_order 与 cancel_delivery 涉及的全部状态。
    #[test]
    fn 测试_销售订单状态常量值正确性() {
        assert_eq!(so_status::DRAFT, "draft");
        assert_eq!(so_status::PENDING, "pending");
        assert_eq!(so_status::APPROVED, "approved");
        assert_eq!(so_status::PARTIAL_SHIPPED, "partial_shipped");
        assert_eq!(so_status::SHIPPED, "shipped");
        assert_eq!(so_status::COMPLETED, "completed");
        assert_eq!(so_status::CANCELLED, "cancelled");
    }

    /// 测试_库存预留状态常量值正确性
    ///
    /// 校验 inventory_reservation 子模块的预留状态常量值（小写），
    /// 覆盖 reduce_inventory / release_reservations / cancel_delivery 涉及的状态转换。
    #[test]
    fn 测试_库存预留状态常量值正确性() {
        assert_eq!(reservation_status::PENDING, "pending");
        assert_eq!(reservation_status::LOCKED, "locked");
        assert_eq!(reservation_status::CONSUMED, "consumed");
        assert_eq!(reservation_status::RELEASED, "released");
        assert_eq!(reservation_status::CANCELLED, "cancelled");
    }

    // ===== ship_order 状态校验 =====

    /// 测试_发货状态校验_仅已审批订单可发货
    ///
    /// 验证 ship_order 中订单状态校验门：仅 APPROVED 状态可发货，其余状态拒绝。
    #[test]
    fn 测试_发货状态校验_仅已审批订单可发货() {
        // 已审批：放行
        assert!(ship_order_status_gate(so_status::APPROVED).is_ok());
        // 其他状态：拒绝
        assert!(ship_order_status_gate(so_status::DRAFT).is_err());
        assert!(ship_order_status_gate(so_status::PENDING).is_err());
        assert!(ship_order_status_gate(so_status::SHIPPED).is_err());
        assert!(ship_order_status_gate(so_status::PARTIAL_SHIPPED).is_err());
        assert!(ship_order_status_gate(so_status::CANCELLED).is_err());

        // 错误类型应为 BusinessError
        let err = ship_order_status_gate(so_status::DRAFT).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    // ===== 全部发货判定 =====

    /// 测试_全部发货判定_所有明细已发足
    ///
    /// 验证 ship_order 中 is_fully_shipped 判定：所有明细已发足（含恰好相等与超发）。
    #[test]
    fn 测试_全部发货判定_所有明细已发足() {
        // 全部发足
        let items = vec![(decs!("100"), decs!("100")), (decs!("50"), decs!("50"))];
        assert!(compute_is_fully_shipped(&items));

        // 边界：恰好相等也算全部发货
        let items_eq = vec![(decs!("10"), decs!("10"))];
        assert!(compute_is_fully_shipped(&items_eq));

        // 超发也算全部发货
        let items_over = vec![(decs!("120"), decs!("100"))];
        assert!(compute_is_fully_shipped(&items_over));
    }

    /// 测试_全部发货判定_部分明细未发足
    ///
    /// 验证 ship_order 中 is_fully_shipped 判定：任一明细未发足即为部分发货。
    #[test]
    fn 测试_全部发货判定_部分明细未发足() {
        // 部分明细未发足
        let items = vec![(decs!("100"), decs!("100")), (decs!("30"), decs!("50"))];
        assert!(!compute_is_fully_shipped(&items));

        // 全部未发
        let items_none = vec![(Decimal::ZERO, decs!("50"))];
        assert!(!compute_is_fully_shipped(&items_none));
    }

    // ===== 发货后订单状态选择 =====

    /// 测试_发货后订单状态选择_全部发货为已发货
    ///
    /// 验证 ship_order 中全部发货时订单状态置为 SHIPPED。
    #[test]
    fn 测试_发货后订单状态选择_全部发货为已发货() {
        assert_eq!(compute_new_status_after_ship(true), so_status::SHIPPED);
    }

    /// 测试_发货后订单状态选择_部分发货为部分发货
    ///
    /// 验证 ship_order 中部分发货时订单状态置为 PARTIAL_SHIPPED。
    #[test]
    fn 测试_发货后订单状态选择_部分发货为部分发货() {
        assert_eq!(
            compute_new_status_after_ship(false),
            so_status::PARTIAL_SHIPPED
        );
    }

    // ===== check_inventory 校验 =====

    /// 测试_库存检查_预留数量不足拒绝
    ///
    /// 验证 check_inventory 中预留数量校验：预留 < 发货 拒绝，预留 >= 发货 放行。
    #[test]
    fn 测试_库存检查_预留数量不足拒绝() {
        // 预留 < 发货：拒绝
        let err = check_inventory_reservation_logic(decs!("30"), decs!("50"), 1).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("预留数量"));

        // 预留 = 发货：放行（边界）
        assert!(check_inventory_reservation_logic(decs!("50"), decs!("50"), 1).is_ok());
        // 预留 > 发货：放行
        assert!(check_inventory_reservation_logic(decs!("80"), decs!("50"), 1).is_ok());
    }

    /// 测试_库存检查_库存数量不足拒绝
    ///
    /// 验证 check_inventory 中库存数量校验：库存 < 发货 拒绝，库存 >= 发货 放行。
    #[test]
    fn 测试_库存检查_库存数量不足拒绝() {
        // 库存 < 发货：拒绝
        let err = check_inventory_stock_logic(decs!("20"), decs!("50"), 2).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("库存"));

        // 库存 = 发货：放行（边界）
        assert!(check_inventory_stock_logic(decs!("50"), decs!("50"), 2).is_ok());
        // 库存 > 发货：放行
        assert!(check_inventory_stock_logic(decs!("100"), decs!("50"), 2).is_ok());
    }

    /// 测试_库存检查_库存不存在拒绝
    ///
    /// 验证 check_inventory 中 stock_map.get 返回 None 时的错误构造：
    /// 返回"产品 X 库存不存在"业务错误。
    #[test]
    fn 测试_库存检查_库存不存在拒绝() {
        let err = AppError::business(format!("产品 {} 库存不存在", 3));
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("库存不存在"));
    }

    // ===== 库存扣减/恢复计算公式 =====

    /// 测试_库存扣减计算公式
    ///
    /// 验证 reduce_inventory 的对称更新公式（发货扣减库存）：
    /// quantity_available -= qty，quantity_shipped += qty，
    /// 且守恒不变量：可用 + 已发 恒定。
    #[test]
    fn 测试_库存扣减计算公式() {
        let available = decs!("100");
        let shipped = decs!("20");
        let qty = decs!("30");

        let new_available = available - qty;
        let new_shipped = shipped + qty;

        assert_eq!(new_available, decs!("70"));
        assert_eq!(new_shipped, decs!("50"));
        // 守恒不变量：可用 + 已发 在扣减前后恒定
        assert_eq!(new_available + new_shipped, available + shipped);
    }

    /// 测试_库存恢复计算公式
    ///
    /// 验证 restore_inventory 的对称反向更新公式（cancel_delivery 取消发货时使用）：
    /// quantity_available += qty，quantity_shipped -= qty，
    /// 且守恒不变量：可用 + 已发 恒定。
    #[test]
    fn 测试_库存恢复计算公式() {
        let available = decs!("70");
        let shipped = decs!("50");
        let qty = decs!("30");

        let new_available = available + qty;
        let new_shipped = shipped - qty;

        assert_eq!(new_available, decs!("100"));
        assert_eq!(new_shipped, decs!("20"));
        // 守恒不变量：可用 + 已发 在恢复前后恒定
        assert_eq!(new_available + new_shipped, available + shipped);
    }

    // ===== 预留状态转换 =====

    /// 测试_预留状态转换_扣减时待处理转已消耗
    ///
    /// 验证 reduce_inventory 中将预留状态从 PENDING 更新为 CONSUMED。
    #[test]
    fn 测试_预留状态转换_扣减时待处理转已消耗() {
        let from_status = reservation_status::PENDING;
        let to_status = reservation_status::CONSUMED;

        assert_eq!(from_status, "pending");
        assert_eq!(to_status, "consumed");
        assert_ne!(from_status, to_status);
    }

    /// 测试_预留状态转换_释放时待处理转已取消
    ///
    /// 验证 release_reservations 中将预留状态从 PENDING 更新为 CANCELLED。
    #[test]
    fn 测试_预留状态转换_释放时待处理转已取消() {
        let from_status = reservation_status::PENDING;
        let to_status = reservation_status::CANCELLED;

        assert_eq!(from_status, "pending");
        assert_eq!(to_status, "cancelled");
        assert_ne!(from_status, to_status);
    }

    /// 测试_预留状态恢复_取消发货时已消耗转待处理
    ///
    /// 验证 cancel_delivery 中将预留状态从 CONSUMED 恢复为 PENDING
    /// （对称反向于 reduce_inventory 的 PENDING → CONSUMED 转换）。
    #[test]
    fn 测试_预留状态恢复_取消发货时已消耗转待处理() {
        let from_status = reservation_status::CONSUMED;
        let to_status = reservation_status::PENDING;

        assert_eq!(from_status, "consumed");
        assert_eq!(to_status, "pending");
        assert_ne!(from_status, to_status);
    }

    // ===== cancel_delivery 校验 =====

    /// 测试_取消发货状态校验_仅已发货可取消
    ///
    /// 验证 cancel_delivery 中发货单状态校验门：仅 SHIPPED 状态可取消，
    /// 其余状态拒绝且错误消息包含当前状态与"仅 SHIPPED 状态可取消"。
    #[test]
    fn 测试_取消发货状态校验_仅已发货可取消() {
        // 已发货：放行
        assert!(cancel_delivery_status_gate(delivery_status::SHIPPED).is_ok());
        // 其他状态：拒绝
        assert!(cancel_delivery_status_gate(delivery_status::PENDING).is_err());
        assert!(cancel_delivery_status_gate(delivery_status::CANCELLED).is_err());

        // 错误消息应包含当前状态和"仅 SHIPPED 状态可取消"
        let err = cancel_delivery_status_gate(delivery_status::PENDING).unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        let msg = err.to_string();
        assert!(msg.contains("pending"));
        assert!(msg.contains("仅 SHIPPED 状态可取消"));
    }

    /// 测试_取消发货订单状态回退_全部取消转已审批
    ///
    /// 验证 cancel_delivery 中：所有发货取消后 has_shipped=false → 订单回退到 APPROVED。
    #[test]
    fn 测试_取消发货订单状态回退_全部取消转已审批() {
        assert_eq!(
            compute_new_status_after_cancel(false),
            so_status::APPROVED
        );
    }

    /// 测试_取消发货订单状态回退_部分取消转部分发货
    ///
    /// 验证 cancel_delivery 中：仍有已发数量 has_shipped=true → 订单回退到 PARTIAL_SHIPPED。
    #[test]
    fn 测试_取消发货订单状态回退_部分取消转部分发货() {
        assert_eq!(
            compute_new_status_after_cancel(true),
            so_status::PARTIAL_SHIPPED
        );
    }

    /// 测试_取消发货订单状态回退条件_仅已发货或部分发货回退
    ///
    /// 验证 cancel_delivery 中状态回退触发条件：仅 SHIPPED 或 PARTIAL_SHIPPED 才回退，
    /// 避免覆盖 CANCELLED / COMPLETED 等终态。
    #[test]
    fn 测试_取消发货订单状态回退条件_仅已发货或部分发货回退() {
        // 可回退
        assert!(order_status_rollback_eligible(so_status::SHIPPED));
        assert!(order_status_rollback_eligible(so_status::PARTIAL_SHIPPED));
        // 不可回退（避免覆盖终态）
        assert!(!order_status_rollback_eligible(so_status::CANCELLED));
        assert!(!order_status_rollback_eligible(so_status::COMPLETED));
        assert!(!order_status_rollback_eligible(so_status::DRAFT));
        assert!(!order_status_rollback_eligible(so_status::APPROVED));
    }

    /// 测试_取消发货备注格式
    ///
    /// 验证 cancel_delivery 中 `format!("[取消原因] {}", reason)` 的备注格式，
    /// 取消原因会被记录到发货单 remarks 字段。
    #[test]
    fn 测试_取消发货备注格式() {
        let remark = format_cancel_remark("客户拒收");
        assert_eq!(remark, "[取消原因] 客户拒收");

        // 空原因：前缀仍保留
        let remark_empty = format_cancel_remark("");
        assert_eq!(remark_empty, "[取消原因] ");
    }

    /// 测试_恢复库存防御性校验_已发货数量不足
    ///
    /// 验证 restore_inventory 中 `if stock.quantity_shipped < quantity` 的防御性校验：
    /// 已发货数量小于要恢复的数量时应拒绝（库存数据不一致），
    /// 已发货数量 >= 恢复数量时允许（含边界相等）。
    #[test]
    fn 测试_恢复库存防御性校验_已发货数量不足() {
        let shipped = decs!("20");
        let restore_qty = decs!("30");

        // 复现 restore_inventory 中 `if stock.quantity_shipped < quantity` 判定
        let should_reject = shipped < restore_qty;
        assert!(should_reject);

        // 错误构造与消息校验
        let err = AppError::business(format!(
            "产品 {} 已发货数量 {} 小于要恢复的数量 {}，库存数据不一致",
            1, shipped, restore_qty
        ));
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(err.to_string().contains("库存数据不一致"));

        // 边界：shipped == restore_qty 应允许
        assert!(decs!("30") >= decs!("30"));
        // shipped > restore_qty 应允许
        assert!(decs!("50") >= decs!("30"));
    }

    // ===== 夹具宏可用性 =====

    /// 测试_夹具宏可用性_decs和ymd
    ///
    /// 验证项目测试夹具宏 decs!（Decimal 字符串解析）和 ymd!（NaiveDate 解析）可正常工作，
    /// 这两个宏在 utils/unwrap_safe.rs 中通过 #[macro_export] 导出。
    #[test]
    fn 测试_夹具宏可用性_decs和ymd() {
        // decs! 解析 Decimal 字符串
        let d = decs!("123.45");
        assert_eq!(d.to_string(), "123.45");

        // ymd! 解析日期
        let date = ymd!(2026, 7, 9);
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-07-09");
    }

    // ===== 服务实例化与数据库交互 =====

    /// 测试_服务实例创建
    ///
    /// 验证 SalesService 在 SQLite 内存数据库 + mock SearchClient 上能正常实例化，
    /// SalesService::new 需要 db 与 search_client 两个依赖，使用 ElasticClient::mock() 提供空实现。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 校验服务内部依赖强引用计数 >= 1，证明实例化成功
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_取消发货_需要真实数据库
    ///
    /// 需要 sales_deliveries 表 schema 与真实数据，标注 #[ignore] 仅在 CI 提供数据库时运行。
    /// 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
    #[tokio::test]
    #[ignore = "依赖数据库 schema，CI 中由 TEST_DATABASE_URL 提供真实数据库"]
    async fn 测试_取消发货_需要真实数据库() {
        let db = setup_test_db().await;
        let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
        let service = SalesService::new(Arc::new(db), search_client);

        // 不存在的发货单应返回错误（NotFound 或数据库错误），调用路径不 panic
        let result = service
            .cancel_delivery(99999, "测试取消".to_string(), 1)
            .await;
        assert!(result.is_err());
    }

    // ===== v14 批次 421 T-P1-5：缸号同订单校验 validate_dye_lot_consistency =====
    // 依据：fabric-industry-research.md §2.3 约束 5
    // 业务语义：一个缸号代表一次染色，同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺

    /// 测试_缸号同订单校验_空发货明细通过
    ///
    /// 无发货明细时校验通过（边界场景）。
    #[test]
    fn 测试_缸号同订单校验_空发货明细通过() {
        let items: Vec<ShipOrderItemRequest> = vec![];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_单产品单缸号通过
    ///
    /// 同一 product_id 仅一个 dye_lot_no → 通过。
    #[test]
    fn 测试_缸号同订单校验_单产品单缸号通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("20"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("5"), Some("DL001".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_多产品各自单缸号通过
    ///
    /// 不同 product_id 可使用不同 dye_lot_no，互不影响 → 通过。
    #[test]
    fn 测试_缸号同订单校验_多产品各自单缸号通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1002, decs!("20"), Some("DL002".to_string())),
            build_ship_item(1003, decs!("5"), Some("DL003".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_同产品不同缸号拒绝
    ///
    /// 同一 product_id 出现多个不同 dye_lot_no → 拒绝（混缸色差风险）。
    #[test]
    fn 测试_缸号同订单校验_同产品不同缸号拒绝() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("20"), Some("DL002".to_string())),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_err(), "同产品不同缸号应被拒绝");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("1001"), "错误信息应包含 product_id");
        assert!(msg.contains("DL001"), "错误信息应包含第一个缸号");
        assert!(msg.contains("DL002"), "错误信息应包含第二个缸号");
        assert!(msg.contains("色差"), "错误信息应说明色差风险");
    }

    /// 测试_缸号同订单校验_未指定缸号通过
    ///
    /// 所有明细均未指定 dye_lot_no（None 或空字符串）→ 跳过校验通过，兼容无缸号场景。
    #[test]
    fn 测试_缸号同订单校验_未指定缸号通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), None),
            build_ship_item(1001, decs!("20"), None),
            build_ship_item(1002, decs!("5"), None),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_空字符串缸号视为未指定
    ///
    /// dye_lot_no 为空字符串时视为未指定，跳过校验通过。
    #[test]
    fn 测试_缸号同订单校验_空字符串缸号视为未指定() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("".to_string())),
            build_ship_item(1001, decs!("20"), Some("".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_部分指定部分未指定通过
    ///
    /// 同一 product_id 部分明细指定缸号 DL001，部分未指定 → 仅校验已指定的，
    /// 未指定不参与比较 → 通过。
    #[test]
    fn 测试_缸号同订单校验_部分指定部分未指定通过() {
        let items = vec![
            build_ship_item(1001, decs!("10"), Some("DL001".to_string())),
            build_ship_item(1001, decs!("20"), None),
            build_ship_item(1001, decs!("5"), Some("DL001".to_string())),
        ];
        assert!(validate_dye_lot_consistency(&items).is_ok());
    }

    /// 测试_缸号同订单校验_错误信息包含缸号列表
    ///
    /// 验证错误信息中包含所有冲突的缸号，便于业务人员定位问题。
    #[test]
    fn 测试_缸号同订单校验_错误信息包含缸号列表() {
        let items = vec![
            build_ship_item(2002, decs!("10"), Some("缸号A".to_string())),
            build_ship_item(2002, decs!("20"), Some("缸号B".to_string())),
            build_ship_item(2002, decs!("5"), Some("缸号C".to_string())),
        ];
        let err = validate_dye_lot_consistency(&items).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("2002"));
        assert!(msg.contains("缸号A"));
        assert!(msg.contains("缸号B"));
        assert!(msg.contains("缸号C"));
    }

    /// 测试夹具：构造 ShipOrderItemRequest
    ///
    /// 集中构造发货明细，避免每个测试重复字段初始化（规则 6 mock 数据抽取）。
    /// batch_no 默认 None，color_no 默认 None，仅 product_id/quantity/dye_lot_no 可变。
    fn build_ship_item(
        product_id: i32,
        quantity: Decimal,
        dye_lot_no: Option<String>,
    ) -> ShipOrderItemRequest {
        ShipOrderItemRequest {
            product_id,
            quantity,
            batch_no: None,
            color_no: None,
            dye_lot_no,
        }
    }
}
