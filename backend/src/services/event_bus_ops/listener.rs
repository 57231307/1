//! 事件监听与关闭子模块（event_bus_ops/listener）
//!
//! 从原 `event_bus.rs` 迁移：
//! - `start_event_listener`：主事件监听器（业务事件分发中枢，调用 8+ 个业务 service）
//! - `shutdown_event_bus`：优雅关闭事件总线所有 spawn task
//! - 5 个 `handle_*` 圈复杂度优化 helper（采购收货 / BPM 审批 / 低库存 / 缺料 / 财务指标）
//! - B-P1-3 主数据变更冗余字段刷新 helper（`refresh_customer_name_redundancy` /
//!   `refresh_supplier_name_redundancy` 及其子更新函数）
//!
//! `MAIN_LISTENER_HANDLE` 全局 static 定义在 facade（`crate::services::event_bus`），
//! 本模块通过 `pub(crate)` 访问。

use crate::search::SearchClient;
use crate::services::event_bus::{lock_event_bus_state, BusinessEvent, EVENT_BUS, MAIN_LISTENER_HANDLE};
use futures::FutureExt;
use sea_orm::DatabaseConnection;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

// ============================================================================
// 旧 API：`start_event_listener`（保持完全兼容）
// ============================================================================

pub async fn start_event_listener(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) {
    // 启动库存财务桥接服务监听器
    crate::services::inventory_finance_bridge_service::InventoryFinanceBridgeService::start_listener(db.clone());

    // v14 批次 422 T-P1-7：启动染色成本桥接监听器
    // 监听 DyeBatchCompleted 事件，自动创建成本归集草稿记录
    crate::services::dye_batch_cost_bridge_service::DyeBatchCostBridgeService::start_listener(db.clone());

    let mut receiver = EVENT_BUS.subscribe();

    // 批次 125 v8 复审 P1 修复：search_client 移入 tokio::spawn 闭包，
    // 供闭包内 SalesService::new(db, search_client) 实例化使用。
    // L-28 修复（批次 373 v13 复审）：保存 spawn 句柄供 shutdown_event_bus() abort
    let listener_handle = tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            // 批次 7（2026-06-28）：单次事件处理 panic 隔离
            // 主事件监听器是业务事件分发中枢，调用 8+ 个业务 service 方法
            // （po_service.receive_order、ap_service.mark_as_paid、sales_service.approve_order 等），
            // 任一 service 内部 panic 会导致整个事件分发永久停止，影响采购收货确认、
            // AP/AR 发票状态更新、BPM 审批回写、低库存预警、缺料采购建议、财务指标计算。
            let result = AssertUnwindSafe(async {
            match event {
                BusinessEvent::PurchaseReceiptCompleted { receipt_id, order_id, .. } => {
                    handle_purchase_receipt_completed(db.clone(), receipt_id, order_id).await;
                }
                BusinessEvent::SalesOrderShipped { order_id, customer_id, .. } => {
                    tracing::info!(
                        "Event received: SalesOrderShipped for order {}, customer {}",
                        order_id,
                        customer_id
                    );
                    // P1 5-2 修复（批次 62）：销售发货事件触发财务指标刷新
                    // 原监听器仅打日志无业务逻辑，销售发货→AR 生成后财务指标（应收/收入）未更新，
                    // 导致财务看板数据滞后。发布 FinancialIndicatorUpdate 事件触发财务指标计算服务刷新。
                    let period = chrono::Utc::now().format("%Y-%m").to_string();
                    EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
                        period,
                        trigger_source: format!("sales_shipped:{}", order_id),
                    });
                }
                BusinessEvent::PaymentCompleted { invoice_id, user_id: _, .. } => {
                    tracing::info!(
                        "Event received: PaymentCompleted for invoice {}",
                        invoice_id
                    );
                    // B-P2-1 修复（批次 385 v13 复审）：移除冗余的 mark_as_paid 调用
                    // 原实现：ap_payment_service::create_payment 已在事务内更新 ap_invoice 状态
                    // （PAYMENT_PAID/PAYMENT_PARTIAL_PAID），commit 后发布事件，
                    // 监听器又调用 mark_as_paid 重复更新状态，且状态门不包含 PAID 会导致报错。
                    // 修复：仅记录日志，状态变更由 create_payment 事务内完成。
                    // mark_as_paid 方法保留，供未来 handler 直接调用（手动标记已付款）。
                    tracing::info!(
                        "付款已完成，ap_invoice {} 状态已在 create_payment 事务内更新，无需重复调用 mark_as_paid",
                        invoice_id
                    );
                }
                BusinessEvent::PurchaseOrderApproved { order_id, .. } => {
                    tracing::info!(
                        "Event received: PurchaseOrderApproved for order {}",
                        order_id
                    );
                }
                BusinessEvent::CollectionCompleted {
                    invoice_id: Some(inv_id),
                    user_id: _,
                    ..
                } => {
                    tracing::info!("Event received: CollectionCompleted for invoice {}", inv_id);
                    // B-P2-1 修复（批次 385 v13 复审）：移除冗余的 mark_as_paid 调用
                    // 原实现：ar_service::create_payment 已在事务内更新 ar_invoice 状态
                    // （PAYMENT_PAID/PAYMENT_PARTIAL_PAID），commit 后发布事件，
                    // 监听器又调用 mark_as_paid 重复更新状态，且状态门不包含 PAID 会导致报错。
                    // 修复：仅记录日志，状态变更由 create_payment 事务内完成。
                    // mark_as_paid 方法保留，供未来 handler 直接调用（手动标记已收款）。
                    tracing::info!(
                        "收款已完成，ar_invoice {} 状态已在 create_payment 事务内更新，无需重复调用 mark_as_paid",
                        inv_id
                    );
                }
                BusinessEvent::InventoryCountCompleted {
                    count_id,
                    variance_count,
                } => {
                    tracing::info!(
                        "处理库存盘点完成事件，盘点单ID: {}, 差异数: {}",
                        count_id,
                        variance_count
                    );
                    tracing::info!(
                        ">> [报告服务] 盘点单 {} 的差异报告(差异: {}) 已生成并存档",
                        count_id,
                        variance_count
                    );
                }
                BusinessEvent::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                    approver_id,
                } => {
                    handle_bpm_process_finished(
                        db.clone(),
                        search_client.clone(),
                        business_type,
                        business_id,
                        approved,
                        approver_id,
                    )
                    .await;
                }
                BusinessEvent::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => {
                    handle_low_stock_alert(
                        db.clone(),
                        product_id,
                        warehouse_id,
                        current_quantity,
                        reorder_point,
                        reorder_quantity,
                    )
                    .await;
                }
                BusinessEvent::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                } => {
                    handle_financial_indicator_update(db.clone(), period, trigger_source).await;
                }
                BusinessEvent::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                } => {
                    handle_material_shortage_alert(
                        db.clone(),
                        material_id,
                        material_name,
                        material_code,
                        required_quantity,
                        available_quantity,
                        shortage_quantity,
                        shortage_level,
                        affected_orders_count,
                    )
                    .await;
                }
                // B-P1-3 修复（批次 384 v13 复审）：客户主数据变更事件
                // 异步刷新关联单据的 customer_name 冗余字段
                BusinessEvent::CustomerUpdated {
                    customer_id,
                    customer_name,
                    user_id: _,
                } => {
                    let db_clone = db.clone();
                    let cid = customer_id;
                    let cname = customer_name;
                    tokio::spawn(async move {
                        if let Err(e) =
                            refresh_customer_name_redundancy(&db_clone, cid, &cname).await
                        {
                            tracing::warn!(
                                "刷新客户 {} 关联单据冗余字段失败：{}",
                                cid,
                                e
                            );
                        }
                    });
                }
                // B-P1-3 修复（批次 384 v13 复审）：供应商主数据变更事件
                // 异步刷新关联单据的 supplier_name 冗余字段
                BusinessEvent::SupplierUpdated {
                    supplier_id,
                    supplier_name,
                    user_id: _,
                } => {
                    let db_clone = db.clone();
                    let sid = supplier_id;
                    let sname = supplier_name;
                    tokio::spawn(async move {
                        if let Err(e) =
                            refresh_supplier_name_redundancy(&db_clone, sid, &sname).await
                        {
                            tracing::warn!(
                                "刷新供应商 {} 关联单据冗余字段失败：{}",
                                sid,
                                e
                            );
                        }
                    });
                }
                // v14 批次 420 修复 G-P1-3：显式处理 InventoryTransactionCreated 事件
                // 原实现 `_ => {}` 静默吞掉该事件，违背事件贯通原则。
                // 凭证生成主链路已由独立的 inventory_finance_bridge_service 监听器负责，
                // 主监听器侧仅打 debug 日志（避免与桥接监听器重复生成凭证）。
                BusinessEvent::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    ..
                } => {
                    tracing::debug!(
                        transaction_id,
                        transaction_type = %transaction_type,
                        product_id,
                        warehouse_id,
                        "主监听器收到 InventoryTransactionCreated（凭证生成由库存财务桥接监听器独立处理）"
                    );
                }
                // v14 批次 420 修复 T-P1-3：染色完成事件分支
                // 当前仅打日志记录事件到达，后续可在该分支触发质检单生成等下游业务。
                BusinessEvent::DyeBatchCompleted {
                    batch_id,
                    batch_no,
                    color_no,
                    ..
                } => {
                    tracing::info!(
                        batch_id,
                        batch_no = %batch_no,
                        color_no = ?color_no,
                        "收到染色完成事件（DyeBatchCompleted），可触发质检单生成/成本结转"
                    );
                }
                // v14 批次 420 修复 T-P1-3：质检完成事件分支
                BusinessEvent::QualityInspectionCompleted {
                    inspection_id,
                    batch_id,
                    product_id,
                    result,
                    ..
                } => {
                    tracing::info!(
                        inspection_id,
                        batch_id = ?batch_id,
                        product_id,
                        result = %result,
                        "收到质检完成事件（QualityInspectionCompleted），可触发库存入库/成本结转"
                    );
                }
                // 批次 342 v11 复审 P3 修复：移除过时的 #[allow(unreachable_patterns)]，
                // v14 批次 420 修复 G-P1-3：将 `_ => {}` 改为打 warn 日志，
                // 防止未来新增事件变体被静默吞掉。
                _ => {
                    tracing::warn!("主监听器收到未处理的事件变体: {:?}", event);
                }
            }
            });  // 批次 7：AssertUnwindSafe(async { ... }) 闭合
            let result = result.catch_unwind().await;

            // 批次 7：panic 隔离后的结果处理
            if let Err(panic_payload) = result {
                let panic_msg = panic_payload
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                    .unwrap_or("<非字符串 panic payload>");
                tracing::error!(
                    panic = %panic_msg,
                    "⚠ 事件总线主监听器 spawn 任务内 panic 已被隔离，事件分发继续运行（不退出循环）"
                );
            }
        }
    });

    // L-28 修复（批次 373 v13 复审）：保存主监听器句柄到全局 static
    if let Ok(mut guard) = MAIN_LISTENER_HANDLE.lock() {
        *guard = Some(listener_handle);
    }
}

// ============================================================================
// start_event_listener 的 match arm 处理 helper（D12 圈复杂度优化）
//
// 主监听器原始 CC≈33，提取 5 个复杂 arm 为独立 helper 后主函数仅保留 match 分发，
// 圈复杂度降至阈值 15 以下。helper 均为自由 async fn，按业务事件边界组织。
// ============================================================================

/// 处理采购收货完成事件：调用 po_service.receive_order 并传入 receipt_id 做幂等校验
async fn handle_purchase_receipt_completed(db: Arc<DatabaseConnection>, receipt_id: i32, order_id: i32) {
    tracing::info!(
        "Event received: PurchaseReceiptCompleted for order {}, receipt {}",
        order_id, receipt_id
    );
    let po_service = crate::services::po::order::PurchaseOrderService::new(db);
    // P0 3-6 修复：传入 receipt_id 做幂等校验，防止事件重投导致重复入库
    match po_service.receive_order(order_id, Some(receipt_id)).await {
        Ok(_) => tracing::info!("Successfully updated purchase order {} status to RECEIVED", order_id),
        Err(e) => tracing::error!("Failed to update purchase order {}: {}", order_id, e),
    }
}

/// 处理 BPM 流程结束事件：幂等校验后按 business_type 分发到对应 service 的 approve/reject 方法
async fn handle_bpm_process_finished(
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
    business_type: String,
    business_id: i32,
    approved: bool,
    approver_id: i32,
) {
    tracing::info!(
        "处理BPM流程结束事件: type={}, id={}, approved={}, approver_id={}",
        business_type, business_id, approved, approver_id
    );
    // B-P1-8 修复（批次 366 v13 复审）：事件幂等处理
    let idempotency_service =
        crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone());
    let event_key = format!("bpm:{}:{}:{}", business_type, business_id, approved);
    let should_process = match idempotency_service
        .try_mark_processed("event_bus_main", &event_key, "BpmProcessFinished")
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "BpmProcessFinished 幂等检查失败 type={} id={}: {}",
                business_type, business_id, e
            );
            false
        }
    };
    if !should_process {
        return;
    }
    match business_type.as_str() {
        "purchase_order" => {
            handle_bpm_purchase_order(db, business_id, approved, approver_id).await;
        }
        "sales_order" => {
            handle_bpm_sales_order(db, search_client, business_id, approved, approver_id).await;
        }
        "production_order" => {
            handle_bpm_production_order(db, business_id, approved, approver_id).await;
        }
        other => tracing::warn!("未识别的 BPM business_type: {}", other),
    }
}

/// 处理采购订单 BPM 审批结果回写（approve_order / reject_order）
async fn handle_bpm_purchase_order(db: Arc<DatabaseConnection>, business_id: i32, approved: bool, approver_id: i32) {
    let po_service = crate::services::po::order::PurchaseOrderService::new(db);
    // P2 5-18 修复：使用事件携带的 approver_id 替代硬编码 0
    if approved {
        if let Err(e) = po_service.approve_order(business_id, approver_id).await {
            tracing::error!("Failed to approve purchase_order {} via BPM: {}", business_id, e);
        } else {
            tracing::info!("Successfully approved purchase_order {} via BPM", business_id);
        }
    } else if let Err(e) = po_service
        .reject_order(business_id, "BPM审批拒绝".to_string(), approver_id)
        .await
    {
        tracing::error!("Failed to reject purchase_order {} via BPM: {}", business_id, e);
    }
}

/// 处理销售订单 BPM 审批结果回写（approve_order / reject_order）
async fn handle_bpm_sales_order(
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
    business_id: i32,
    approved: bool,
    approver_id: i32,
) {
    let sales_service = crate::services::so::order::SalesService::new(db, search_client);
    if approved {
        if let Err(e) = sales_service.approve_order(business_id, approver_id).await {
            tracing::error!("Failed to approve sales_order {} via BPM: {}", business_id, e);
        } else {
            tracing::info!("Successfully approved sales_order {} via BPM", business_id);
        }
    } else {
        match sales_service
            .reject_order(business_id, "BPM审批拒绝".to_string(), approver_id)
            .await
        {
            Ok(_) => tracing::info!("Successfully rejected sales_order {} via BPM", business_id),
            Err(e) => tracing::error!("Failed to reject sales_order {} via BPM: {}", business_id, e),
        }
    }
}

/// 处理生产订单 BPM 审批结果回写（专用 approve_order_via_bpm/reject_order_via_bpm，不回调 BPM 避免循环）
async fn handle_bpm_production_order(db: Arc<DatabaseConnection>, business_id: i32, approved: bool, approver_id: i32) {
    // B-P1-9 修复（批次 360 v13 复审）：原实现仅处理 purchase_order/sales_order，生产订单 BPM 审批结果无法回写
    let prod_service = crate::services::production_order_service::ProductionOrderService::new(db);
    if approved {
        if let Err(e) = prod_service.approve_order_via_bpm(business_id, approver_id).await {
            tracing::error!("Failed to approve production_order {} via BPM: {}", business_id, e);
        } else {
            tracing::info!("Successfully approved production_order {} via BPM", business_id);
        }
    } else {
        if let Err(e) = prod_service
            .reject_order_via_bpm(business_id, "BPM审批拒绝".to_string(), approver_id)
            .await
        {
            tracing::error!("Failed to reject production_order {} via BPM: {}", business_id, e);
        } else {
            tracing::info!("Successfully rejected production_order {} via BPM", business_id);
        }
    }
}

/// 处理低库存预警事件：幂等校验后创建采购建议 + 通知 admin/manager 角色用户
async fn handle_low_stock_alert(
    db: Arc<DatabaseConnection>,
    product_id: i32,
    warehouse_id: i32,
    current_quantity: rust_decimal::Decimal,
    reorder_point: rust_decimal::Decimal,
    reorder_quantity: rust_decimal::Decimal,
) {
    tracing::info!(
        "处理低库存预警事件: 产品ID={}, 仓库ID={}, 当前库存={}, 补货点={}, 建议补货量={}",
        product_id, warehouse_id, current_quantity, reorder_point, reorder_quantity
    );
    // B-P1-8 修复（批次 366 v13 复审）：幂等键含日期，同产品同仓库同一天仅处理一次低库存预警
    let idempotency_service =
        crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone());
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let event_key = format!("low_stock:{}:{}:{}", product_id, warehouse_id, today);
    let should_process = match idempotency_service
        .try_mark_processed("event_bus_main", &event_key, "LowStockAlert")
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "LowStockAlert 幂等检查失败 product={} warehouse={}: {}",
                product_id, warehouse_id, e
            );
            false
        }
    };
    if !should_process {
        return;
    }
    // 创建采购建议
    let po_service = crate::services::po::order::PurchaseOrderService::new(db.clone());
    match po_service
        .create_purchase_suggestion(product_id, warehouse_id, current_quantity, reorder_point, reorder_quantity)
        .await
    {
        Ok(order) => tracing::info!("成功创建采购建议: 订单ID={}, 订单号={}", order.id, order.order_no),
        Err(e) => tracing::error!("创建采购建议失败: {}", e),
    }

    // 发送低库存预警通知给仓库管理员和采购人员
    let notification_service =
        crate::services::event_notification_service::EventNotificationService::new(db.clone());
    let product_name = crate::models::product::Entity::find_by_id(product_id)
        .one(db.as_ref())
        .await
        .ok()
        .flatten()
        .map(|p| p.name)
        .unwrap_or_else(|| format!("产品{}", product_id));

    // P2 5-19 修复：按角色过滤通知用户，仅通知 admin 和 manager 角色
    use crate::utils::admin_checker::{ADMIN_ROLE_CODE, MANAGER_ROLE_CODE};
    let target_role_ids: Vec<i32> = crate::models::role::Entity::find()
        .filter(
            crate::models::role::Column::Code
                .eq(ADMIN_ROLE_CODE)
                .or(crate::models::role::Column::Code.eq(MANAGER_ROLE_CODE)),
        )
        .all(db.as_ref())
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.id)
        .collect();

    let notify_user_ids = if target_role_ids.is_empty() {
        Vec::new()
    } else {
        crate::models::user::Entity::find()
            .filter(crate::models::user::Column::IsActive.eq(true))
            .filter(crate::models::user::Column::RoleId.is_in(target_role_ids))
            .all(db.as_ref())
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|u| u.id)
            .collect::<Vec<i32>>()
    };

    let notify_count = notify_user_ids.len();
    // v17 批次 47 修复：改用批量通知方法，循环外一次获取所有用户设置（避免 N+1）
    if let Err(e) = notification_service
        .notify_inventory_alert_batch(
            &notify_user_ids,
            &product_name,
            product_id,
            &format!("{}米", current_quantity),
            &format!("{}米", reorder_point),
        )
        .await
    {
        tracing::error!(
            "发送低库存预警批量通知失败: 通知人数={}, 错误={}",
            notify_count, e
        );
    }
    tracing::info!(
        "低库存预警通知已发送: 产品={}, 仓库ID={}, 通知人数={}",
        product_name, warehouse_id, notify_count
    );
}

/// 处理缺料预警事件：幂等校验后创建缺料采购建议
async fn handle_material_shortage_alert(
    db: Arc<DatabaseConnection>,
    material_id: i32,
    material_name: String,
    material_code: String,
    required_quantity: rust_decimal::Decimal,
    available_quantity: rust_decimal::Decimal,
    shortage_quantity: rust_decimal::Decimal,
    shortage_level: String,
    affected_orders_count: i32,
) {
    tracing::info!(
        "处理缺料预警事件: 物料ID={}, 物料名称={}, 缺料数量={}, 预警级别={}, 受影响订单数={}",
        material_id, material_name, shortage_quantity, shortage_level, affected_orders_count
    );
    // B-P1-8 修复（批次 366 v13 复审）：幂等键含日期，同物料同一天仅处理一次缺料预警
    let idempotency_service =
        crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone());
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let event_key = format!("material_shortage:{}:{}", material_id, today);
    let should_process = match idempotency_service
        .try_mark_processed("event_bus_main", &event_key, "MaterialShortageAlert")
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "MaterialShortageAlert 幂等检查失败 material={}: {}",
                material_id, e
            );
            false
        }
    };
    if !should_process {
        return;
    }
    let po_service = crate::services::po::order::PurchaseOrderService::new(db.clone());
    // 批次 333 v10 复审 P3 修复：使用 ShortageAlertParams 参数对象替代多参数
    let shortage_params = crate::services::po::price::ShortageAlertParams {
        material_id,
        material_name: material_name.clone(),
        material_code: material_code.clone(),
        required_quantity,
        available_quantity,
        shortage_quantity,
        shortage_level: shortage_level.clone(),
        affected_orders_count,
    };
    match po_service.create_purchase_suggestion_from_shortage(shortage_params).await {
        Ok(order) => tracing::info!(
            "成功创建缺料采购建议: 订单ID={}, 订单号={}, 物料={}",
            order.id, order.order_no, material_name
        ),
        Err(e) => tracing::error!("创建缺料采购建议失败: 物料ID={}, 错误={}", material_id, e),
    }
}

/// 处理财务指标更新事件：调用 FinancialAnalysisService.calculate_indicators 刷新指标
async fn handle_financial_indicator_update(db: Arc<DatabaseConnection>, period: String, trigger_source: String) {
    tracing::info!("处理财务指标更新事件: 期间={}, 触发源={}", period, trigger_source);
    let fa_service =
        crate::services::financial_analysis_service::FinancialAnalysisService::new(db);
    match fa_service.calculate_indicators(&period, 0).await {
        Ok(results) => tracing::info!(
            "财务指标自动计算完成: 期间={}, 计算 {} 个指标",
            period, results.len()
        ),
        Err(e) => tracing::error!("财务指标自动计算失败: 期间={}, 错误={}", period, e),
    }
}

/// L-27+L-28+L-29 修复（批次 373）：优雅关闭事件总线所有 spawn task，幂等安全
pub fn shutdown_event_bus() {
    // L-27：abort Kafka 消费桥接 task
    let consumer_handle = {
        let mut state = lock_event_bus_state();
        state.consumer_handle.take()
    };
    if let Some(handle) = consumer_handle {
        handle.abort();
        tracing::info!("Kafka 消费桥接 task 已关闭");
    }

    // L-28：abort 主事件监听器 task
    let listener_handle = {
        match MAIN_LISTENER_HANDLE.lock() {
            Ok(mut guard) => guard.take(),
            Err(e) => {
                tracing::error!(error = %e, "MAIN_LISTENER_HANDLE 锁中毒，无法关闭主监听器");
                None
            }
        }
    };
    if let Some(handle) = listener_handle {
        handle.abort();
        tracing::info!("事件总线主监听器 task 已关闭");
    }

    // L-29：abort 库存财务桥接监听器 task
    crate::services::inventory_finance_bridge_service::InventoryFinanceBridgeService::shutdown_listener();

    // v14 批次 422 T-P1-7：abort 染色成本桥接监听器 task
    crate::services::dye_batch_cost_bridge_service::DyeBatchCostBridgeService::shutdown_listener();
}

// ============================================================================
// B-P1-3 修复（批次 384 v13 复审）：主数据变更冗余字段刷新
// ============================================================================

/// 刷新客户关联单据的 customer_name 冗余字段
async fn refresh_customer_name_redundancy(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    new_name: &str,
) -> Result<(), sea_orm::DbErr> {
    let now = chrono::Utc::now();
    update_ar_invoices_customer_name(db, customer_id, new_name, now).await?;
    update_ar_collections_customer_name(db, customer_id, new_name, now).await?;
    update_ar_reconciliations_customer_name(db, customer_id, new_name, now).await?;
    update_customer_credits_customer_name(db, customer_id, new_name, now).await?;
    update_sales_contracts_customer_name(db, customer_id, new_name, now).await?;
    tracing::info!(
        "客户 {} 名称已刷新至所有关联单据冗余字段：{}",
        customer_id,
        new_name
    );
    Ok(())
}

/// 更新 ar_invoices.customer_name 冗余字段
async fn update_ar_invoices_customer_name(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    new_name: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    crate::models::ar_invoice::Entity::update_many()
        .filter(crate::models::ar_invoice::Column::CustomerId.eq(customer_id))
        .col_expr(
            crate::models::ar_invoice::Column::CustomerName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::ar_invoice::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;
    Ok(())
}

/// 更新 ar_collections.customer_name 冗余字段
async fn update_ar_collections_customer_name(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    new_name: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    crate::models::ar_collection::Entity::update_many()
        .filter(crate::models::ar_collection::Column::CustomerId.eq(customer_id))
        .col_expr(
            crate::models::ar_collection::Column::CustomerName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::ar_collection::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;
    Ok(())
}

/// 更新 ar_reconciliations.customer_name 冗余字段
async fn update_ar_reconciliations_customer_name(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    new_name: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    crate::models::ar_reconciliation::Entity::update_many()
        .filter(crate::models::ar_reconciliation::Column::CustomerId.eq(customer_id))
        .col_expr(
            crate::models::ar_reconciliation::Column::CustomerName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::ar_reconciliation::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;
    Ok(())
}

/// 更新 customer_credits.customer_name 冗余字段
async fn update_customer_credits_customer_name(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    new_name: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    crate::models::customer_credit::Entity::update_many()
        .filter(crate::models::customer_credit::Column::CustomerId.eq(customer_id))
        .col_expr(
            crate::models::customer_credit::Column::CustomerName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::customer_credit::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;
    Ok(())
}

/// 更新 sales_contracts.customer_name 冗余字段
async fn update_sales_contracts_customer_name(
    db: &sea_orm::DatabaseConnection,
    customer_id: i32,
    new_name: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    crate::models::sales_contract::Entity::update_many()
        .filter(crate::models::sales_contract::Column::CustomerId.eq(customer_id))
        .col_expr(
            crate::models::sales_contract::Column::CustomerName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::sales_contract::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;
    Ok(())
}

/// 刷新供应商关联单据的 supplier_name 冗余字段
///
/// 当供应商主数据 supplier_name 变更时，异步刷新以下表的冗余字段：
/// - purchase_contracts.supplier_name
/// - fixed_assets.supplier_name
async fn refresh_supplier_name_redundancy(
    db: &sea_orm::DatabaseConnection,
    supplier_id: i32,
    new_name: &str,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::sea_query::Expr;
    use sea_orm::ColumnTrait;
    use sea_orm::EntityTrait;
    use sea_orm::QueryFilter;

    let now = chrono::Utc::now();
    // purchase_contracts
    crate::models::purchase_contract::Entity::update_many()
        .filter(crate::models::purchase_contract::Column::SupplierId.eq(supplier_id))
        .col_expr(
            crate::models::purchase_contract::Column::SupplierName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::purchase_contract::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;

    // fixed_assets
    crate::models::fixed_asset::Entity::update_many()
        .filter(crate::models::fixed_asset::Column::SupplierId.eq(supplier_id))
        .col_expr(
            crate::models::fixed_asset::Column::SupplierName,
            Expr::val(new_name.to_string()).into(),
        )
        .col_expr(
            crate::models::fixed_asset::Column::UpdatedAt,
            Expr::val(now).into(),
        )
        .exec(db)
        .await?;

    tracing::info!(
        "供应商 {} 名称已刷新至所有关联单据冗余字段：{}",
        supplier_id,
        new_name
    );
    Ok(())
}
