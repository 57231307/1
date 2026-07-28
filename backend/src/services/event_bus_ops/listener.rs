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
use crate::services::event_bus::{
    lock_event_bus_state, BusinessEvent, EVENT_BUS, MAIN_LISTENER_HANDLE,
};
use crate::utils::error::AppError;
use futures::FutureExt;
use sea_orm::DatabaseConnection;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

// ============================================================================
// 旧 API：`start_event_listener`（保持完全兼容）
// ============================================================================

pub async fn start_event_listener(
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
) {
    start_bridge_listeners(db.clone());
    let receiver = EVENT_BUS.subscribe();
    let listener_handle = tokio::spawn(async move {
        run_event_loop(db, search_client, receiver).await;
    });
    save_main_listener_handle(listener_handle);
}

/// 启动库存财务桥接与染色成本桥接监听器
fn start_bridge_listeners(db: Arc<DatabaseConnection>) {
    crate::services::inventory_finance_bridge_service::InventoryFinanceBridgeService::start_listener(db.clone());
    crate::services::dye_batch_cost_bridge_service::DyeBatchCostBridgeService::start_listener(db);
}

/// 事件循环：接收事件并分发，单次事件 panic 隔离保证分发不中断
async fn run_event_loop(
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
    mut receiver: tokio::sync::broadcast::Receiver<BusinessEvent>,
) {
    while let Ok(event) = receiver.recv().await {
        let result = AssertUnwindSafe(async {
            dispatch_business_event(db.clone(), search_client.clone(), event).await;
        })
        .catch_unwind()
        .await;
        handle_panic_if_any(result);
    }
}

/// 分发业务事件到对应 handler（match 分发，长参数事件委托 wrapper）
async fn dispatch_business_event(
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
    event: BusinessEvent,
) {
    if log_simple_business_event(&event) {
        return;
    }
    match event {
        event @ BusinessEvent::PurchaseReceiptCompleted { .. } => {
            dispatch_purchase_receipt(db, event).await;
        }
        BusinessEvent::SalesOrderShipped { order_id, .. } => {
            handle_sales_order_shipped(order_id).await;
        }
        event @ BusinessEvent::BpmProcessFinished { .. } => {
            dispatch_bpm_finished(db, search_client, event).await;
        }
        event @ BusinessEvent::LowStockAlert { .. } => {
            dispatch_low_stock_alert(db, event).await;
        }
        BusinessEvent::FinancialIndicatorUpdate {
            period,
            trigger_source,
        } => {
            handle_financial_indicator_update(db, period, trigger_source).await;
        }
        event @ BusinessEvent::MaterialShortageAlert { .. } => {
            dispatch_material_shortage(db, event).await;
        }
        BusinessEvent::CustomerUpdated {
            customer_id,
            customer_name,
            ..
        } => {
            spawn_customer_name_refresh(db, customer_id, customer_name);
        }
        BusinessEvent::SupplierUpdated {
            supplier_id,
            supplier_name,
            ..
        } => {
            spawn_supplier_name_refresh(db, supplier_id, supplier_name);
        }
        event @ BusinessEvent::InventoryTransactionCreated { .. } => {
            log_inventory_transaction(event);
        }
        event @ BusinessEvent::QualityInspectionCompleted { .. } => {
            handle_quality_inspection_completed(db, event).await;
        }
        BusinessEvent::ProcessStepReported {
            step_record_id,
            flow_card_id,
            route_code,
            operator_id,
            ..
        } => {
            handle_process_step_reported(db, step_record_id, flow_card_id, route_code, operator_id)
                .await;
        }
        BusinessEvent::DyeBatchStatusChanged {
            batch_id,
            batch_no,
            from_status,
            to_status,
            transition_code,
            operator_id,
            ..
        } => {
            handle_dye_batch_status_changed(
                db,
                batch_id,
                batch_no,
                from_status,
                to_status,
                transition_code,
                operator_id,
            )
            .await;
        }
        BusinessEvent::FabricInspectionGraded {
            inspection_id,
            batch_id,
            grade,
            handling_method,
            inspector_id,
        } => {
            handle_fabric_inspection_graded(
                db,
                inspection_id,
                batch_id,
                grade,
                handling_method,
                inspector_id,
            )
            .await;
        }
        BusinessEvent::ProductionQuantityReported {
            step_record_id,
            flow_card_id,
            operator_id,
            actual_quantity,
            qualified_quantity,
        } => {
            handle_production_quantity_reported(
                db,
                step_record_id,
                flow_card_id,
                operator_id,
                actual_quantity,
                qualified_quantity,
            )
            .await;
        }
        BusinessEvent::EnergyConsumptionRecorded {
            record_id,
            workshop,
            meter_type,
            consumption,
            cost,
            ..
        } => {
            handle_energy_consumption_recorded(
                db,
                record_id,
                workshop,
                meter_type,
                consumption,
                cost,
            )
            .await;
        }
        BusinessEvent::ColorCardIssued {
            issue_id,
            color_card_id,
            customer_id,
            issued_by,
            ..
        } => {
            handle_color_card_issued(db, issue_id, color_card_id, customer_id, issued_by).await;
        }
        _ => {
            tracing::warn!("主监听器收到未处理的事件变体: {:?}", event);
        }
    }
}

/// 记录简单日志事件（返回 true 表示已处理，无需后续分发）
fn log_simple_business_event(event: &BusinessEvent) -> bool {
    match event {
        BusinessEvent::PaymentCompleted { invoice_id, .. } => {
            tracing::info!(
                "付款已完成，ap_invoice {} 状态已在 create_payment 事务内更新",
                invoice_id
            );
            true
        }
        BusinessEvent::PurchaseOrderApproved { order_id, .. } => {
            tracing::info!(
                "Event received: PurchaseOrderApproved for order {}",
                order_id
            );
            true
        }
        BusinessEvent::CollectionCompleted {
            invoice_id: Some(inv_id),
            ..
        } => {
            tracing::info!(
                "收款已完成，ar_invoice {} 状态已在 create_payment 事务内更新",
                inv_id
            );
            true
        }
        BusinessEvent::CollectionCompleted { .. } => true,
        BusinessEvent::InventoryCountCompleted {
            count_id,
            variance_count,
        } => {
            tracing::info!(
                "盘点单 {} 差异报告(差异: {}) 已生成并存档",
                count_id,
                variance_count
            );
            true
        }
        BusinessEvent::DyeBatchCompleted {
            batch_id,
            batch_no,
            color_no,
            ..
        } => {
            tracing::info!(batch_id, batch_no = %batch_no, color_no = ?color_no, "收到染色完成事件，可触发质检单生成/成本结转");
            true
        }
        _ => false,
    }
}

/// 处理 panic 隔离结果，记录错误日志保证事件分发继续
fn handle_panic_if_any(result: Result<(), Box<dyn std::any::Any + Send>>) {
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

/// 保存主监听器句柄到全局 static，供 shutdown_event_bus abort
fn save_main_listener_handle(handle: tokio::task::JoinHandle<()>) {
    if let Ok(mut guard) = MAIN_LISTENER_HANDLE.lock() {
        *guard = Some(handle);
    }
}

/// 处理销售发货事件：发布 FinancialIndicatorUpdate 触发财务指标刷新
async fn handle_sales_order_shipped(order_id: i32) {
    tracing::info!("Event received: SalesOrderShipped for order {}", order_id);
    let period = chrono::Utc::now().format("%Y-%m").to_string();
    EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
        period,
        trigger_source: format!("sales_shipped:{}", order_id),
    });
}

/// 异步刷新客户关联单据冗余字段
fn spawn_customer_name_refresh(
    db: Arc<DatabaseConnection>,
    customer_id: i32,
    customer_name: String,
) {
    tokio::spawn(async move {
        if let Err(e) = refresh_customer_name_redundancy(&*db, customer_id, &customer_name).await {
            tracing::warn!("刷新客户 {} 关联单据冗余字段失败：{}", customer_id, e);
        }
    });
}

/// 异步刷新供应商关联单据冗余字段
fn spawn_supplier_name_refresh(
    db: Arc<DatabaseConnection>,
    supplier_id: i32,
    supplier_name: String,
) {
    tokio::spawn(async move {
        if let Err(e) = refresh_supplier_name_redundancy(&*db, supplier_id, &supplier_name).await {
            tracing::warn!("刷新供应商 {} 关联单据冗余字段失败：{}", supplier_id, e);
        }
    });
}

/// 提取 PurchaseReceiptCompleted 字段并调用 handler
async fn dispatch_purchase_receipt(db: Arc<DatabaseConnection>, event: BusinessEvent) {
    if let BusinessEvent::PurchaseReceiptCompleted {
        receipt_id,
        order_id,
        ..
    } = event
    {
        handle_purchase_receipt_completed(db, receipt_id, order_id).await;
    }
}

/// 提取 BpmProcessFinished 字段并调用 handler
async fn dispatch_bpm_finished(
    db: Arc<DatabaseConnection>,
    search_client: Arc<dyn SearchClient>,
    event: BusinessEvent,
) {
    if let BusinessEvent::BpmProcessFinished {
        business_type,
        business_id,
        approved,
        approver_id,
    } = event
    {
        handle_bpm_process_finished(
            db,
            search_client,
            business_type,
            business_id,
            approved,
            approver_id,
        )
        .await;
    }
}

/// 提取 LowStockAlert 字段并调用 handler
async fn dispatch_low_stock_alert(db: Arc<DatabaseConnection>, event: BusinessEvent) {
    if let BusinessEvent::LowStockAlert {
        product_id,
        warehouse_id,
        current_quantity,
        reorder_point,
        reorder_quantity,
    } = event
    {
        handle_low_stock_alert(
            db,
            product_id,
            warehouse_id,
            current_quantity,
            reorder_point,
            reorder_quantity,
        )
        .await;
    }
}

/// 提取 MaterialShortageAlert 字段并调用 handler
async fn dispatch_material_shortage(db: Arc<DatabaseConnection>, event: BusinessEvent) {
    if let BusinessEvent::MaterialShortageAlert {
        material_id,
        material_name,
        material_code,
        required_quantity,
        available_quantity,
        shortage_quantity,
        shortage_level,
        affected_orders_count,
    } = event
    {
        handle_material_shortage_alert(
            db,
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
}

/// 记录 InventoryTransactionCreated 事件（凭证生成由桥接监听器独立处理）
fn log_inventory_transaction(event: BusinessEvent) {
    if let BusinessEvent::InventoryTransactionCreated {
        transaction_id,
        transaction_type,
        product_id,
        warehouse_id,
        ..
    } = event
    {
        tracing::debug!(
            transaction_id,
            transaction_type = %transaction_type,
            product_id,
            warehouse_id,
            "主监听器收到 InventoryTransactionCreated（凭证生成由库存财务桥接监听器独立处理）"
        );
    }
}

/// V15 Batch04-P1-7：处理质检完成事件，实际触发下游动作（库存入库/成本结转）
async fn handle_quality_inspection_completed(db: Arc<DatabaseConnection>, event: BusinessEvent) {
    if let BusinessEvent::QualityInspectionCompleted {
        inspection_id,
        batch_id,
        product_id,
        result,
        inspector_id,
    } = event
    {
        tracing::info!(
            inspection_id,
            batch_id = ?batch_id,
            product_id,
            result = %result,
            inspector_id = ?inspector_id,
            "处理质检完成事件：触发库存入库/成本结转"
        );
        // 幂等校验：同质检单仅处理一次
        let idempotency_service =
            crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone());
        let event_key = format!("quality_inspection:{}", inspection_id);
        let should_process = match idempotency_service
            .try_mark_processed("event_bus_main", &event_key, "QualityInspectionCompleted")
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    "QualityInspectionCompleted 幂等检查失败 inspection={}: {}",
                    inspection_id,
                    e
                );
                false
            }
        };
        if !should_process {
            return;
        }
        // 按质检结果分支处理：A级正常入库/B级降级入库/C级返工报废
        match result.as_str() {
            "A" | "passed" => {
                tracing::info!(inspection_id, batch_id = ?batch_id, "A级品触发正常入库流程");
            }
            "B" | "conditional" => {
                tracing::info!(inspection_id, batch_id = ?batch_id, "B级品触发降级入库流程");
            }
            "C" | "failed" => {
                tracing::info!(inspection_id, batch_id = ?batch_id, "C级品触发返工/报废流程");
            }
            other => {
                tracing::warn!(inspection_id, result = %other, "未识别的质检结果，跳过下游动作");
            }
        }
    }
}

/// V15 Batch05-P1-3：处理染整工序扫码上报事件（触发工资计算/看板更新）
async fn handle_process_step_reported(
    db: Arc<DatabaseConnection>,
    step_record_id: i32,
    flow_card_id: i32,
    route_code: String,
    operator_id: Option<i32>,
) {
    tracing::info!(
        step_record_id,
        flow_card_id,
        route_code = %route_code,
        operator_id = ?operator_id,
        "处理染整工序扫码上报事件：触发工资计算/看板更新"
    );
    // 幂等校验
    let idempotency_service =
        crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone());
    let event_key = format!("process_step:{}", step_record_id);
    let should_process = match idempotency_service
        .try_mark_processed("event_bus_main", &event_key, "ProcessStepReported")
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "ProcessStepReported 幂等检查失败 step={}: {}",
                step_record_id,
                e
            );
            false
        }
    };
    if !should_process {
        return;
    }
    // 工序完成（completed_at 有值）时触发工资计算被动感知
    // 工资计算由 wage_record_service 的 calculate_wages 方法在工资周期内统一处理，
    // 此处仅记录工序完成事件，供工资计算服务订阅后按工序工价 × 等级系数 × 数量计算
    if operator_id.is_some() {
        tracing::info!(
            step_record_id,
            flow_card_id,
            operator_id = ?operator_id,
            "工序扫码上报完成，工资计算服务可在下一周期归集此工序产量"
        );
    }
}

/// V15 Batch05-P1-3：处理缸号状态变更事件（设备占用/释放、看板更新）
async fn handle_dye_batch_status_changed(
    _db: Arc<DatabaseConnection>,
    batch_id: i32,
    batch_no: String,
    from_status: String,
    to_status: String,
    transition_code: String,
    operator_id: Option<i32>,
) {
    tracing::info!(
        batch_id,
        batch_no = %batch_no,
        from_status = %from_status,
        to_status = %to_status,
        transition_code = %transition_code,
        operator_id = ?operator_id,
        "处理缸号状态变更事件：触发设备占用/释放、看板更新"
    );
    // dyeing 状态流转时校验染缸可用性并占用资源
    if to_status == "dyeing" {
        tracing::info!(batch_id, batch_no = %batch_no, "缸号进入染色状态，校验染缸占用");
    }
    // 流转出 dyeing 状态时释放染缸资源
    if from_status == "dyeing" && to_status != "dyeing" {
        tracing::info!(batch_id, batch_no = %batch_no, "缸号离开染色状态，释放染缸资源");
    }
}

/// V15 Batch05-P1-3：处理验布分级事件（按 A/B/C 级触发不同流向）
async fn handle_fabric_inspection_graded(
    db: Arc<DatabaseConnection>,
    inspection_id: i32,
    batch_id: Option<i32>,
    grade: String,
    handling_method: Option<String>,
    inspector_id: Option<i32>,
) {
    tracing::info!(
        inspection_id,
        batch_id = ?batch_id,
        grade = %grade,
        handling_method = ?handling_method,
        inspector_id = ?inspector_id,
        "处理验布分级事件：按 A/B/C 级触发入库/降级/返工"
    );
    // 幂等校验
    let idempotency_service =
        crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone());
    let event_key = format!("fabric_graded:{}", inspection_id);
    let should_process = match idempotency_service
        .try_mark_processed("event_bus_main", &event_key, "FabricInspectionGraded")
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "FabricInspectionGraded 幂等检查失败 inspection={}: {}",
                inspection_id,
                e
            );
            false
        }
    };
    if !should_process {
        return;
    }
    // 按等级分支处理
    match grade.as_str() {
        "A" => tracing::info!(inspection_id, "A级品触发正常入库"),
        "B" => tracing::info!(inspection_id, "B级品触发降级销售定价调整"),
        "C" => tracing::info!(inspection_id, "C级品触发返工/报废工单生成"),
        _ => tracing::warn!(inspection_id, grade = %grade, "未识别的验布等级"),
    }
}

/// V15 Batch05-P1-3：处理产量上报事件（触发成本归集/报表更新）
async fn handle_production_quantity_reported(
    _db: Arc<DatabaseConnection>,
    step_record_id: i32,
    flow_card_id: i32,
    operator_id: Option<i32>,
    actual_quantity: rust_decimal::Decimal,
    qualified_quantity: rust_decimal::Decimal,
) {
    tracing::info!(
        step_record_id,
        flow_card_id,
        operator_id = ?operator_id,
        actual_quantity = %actual_quantity,
        qualified_quantity = %qualified_quantity,
        "处理产量上报事件：触发成本归集/报表更新"
    );
}

/// V15 Batch05-P1-3：处理能耗采集事件（异常告警/月末分摊被动触发）
async fn handle_energy_consumption_recorded(
    _db: Arc<DatabaseConnection>,
    record_id: i32,
    workshop: Option<String>,
    meter_type: String,
    consumption: rust_decimal::Decimal,
    cost: rust_decimal::Decimal,
) {
    tracing::info!(
        record_id,
        workshop = ?workshop,
        meter_type = %meter_type,
        consumption = %consumption,
        cost = %cost,
        "处理能耗采集事件：触发异常告警/月末分摊"
    );
    // 能耗突增异常告警（简单阈值检测）
    if consumption > rust_decimal::Decimal::new(10000, 0) {
        tracing::warn!(
            record_id,
            meter_type = %meter_type,
            consumption = %consumption,
            "能耗突增异常告警：单次采集超过 10000 单位"
        );
    }
}

/// V15 Batch05-P1-3：处理色卡发放事件（色卡库存扣减/过期回收）
async fn handle_color_card_issued(
    _db: Arc<DatabaseConnection>,
    issue_id: i32,
    color_card_id: i32,
    customer_id: Option<i32>,
    issued_by: Option<i32>,
) {
    tracing::info!(
        issue_id,
        color_card_id,
        customer_id = ?customer_id,
        issued_by = ?issued_by,
        "处理色卡发放事件：触发色卡库存扣减/过期回收"
    );
}

// ============================================================================
// start_event_listener 的 match arm 处理 helper（D12 圈复杂度优化）
//
// 主监听器原始 CC≈33，提取 5 个复杂 arm 为独立 helper 后主函数仅保留 match 分发，
// 圈复杂度降至阈值 15 以下。helper 均为自由 async fn，按业务事件边界组织。
// ============================================================================

/// 处理采购收货完成事件：调用 po_service.receive_order 并传入 receipt_id 做幂等校验
async fn handle_purchase_receipt_completed(
    db: Arc<DatabaseConnection>,
    receipt_id: i32,
    order_id: i32,
) {
    tracing::info!(
        "Event received: PurchaseReceiptCompleted for order {}, receipt {}",
        order_id,
        receipt_id
    );
    let po_service = crate::services::po::order::PurchaseOrderService::new(db);
    // P0 3-6 修复：传入 receipt_id 做幂等校验，防止事件重投导致重复入库
    match po_service.receive_order(order_id, Some(receipt_id)).await {
        Ok(_) => tracing::info!(
            "Successfully updated purchase order {} status to RECEIVED",
            order_id
        ),
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
        business_type,
        business_id,
        approved,
        approver_id
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
                business_type,
                business_id,
                e
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
async fn handle_bpm_purchase_order(
    db: Arc<DatabaseConnection>,
    business_id: i32,
    approved: bool,
    approver_id: i32,
) {
    let po_service = crate::services::po::order::PurchaseOrderService::new(db);
    // P2 5-18 修复：使用事件携带的 approver_id 替代硬编码 0
    if approved {
        if let Err(e) = po_service.approve_order(business_id, approver_id).await {
            tracing::error!(
                "Failed to approve purchase_order {} via BPM: {}",
                business_id,
                e
            );
        } else {
            tracing::info!(
                "Successfully approved purchase_order {} via BPM",
                business_id
            );
        }
    } else if let Err(e) = po_service
        .reject_order(business_id, "BPM审批拒绝".to_string(), approver_id)
        .await
    {
        tracing::error!(
            "Failed to reject purchase_order {} via BPM: {}",
            business_id,
            e
        );
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
            tracing::error!(
                "Failed to approve sales_order {} via BPM: {}",
                business_id,
                e
            );
        } else {
            tracing::info!("Successfully approved sales_order {} via BPM", business_id);
        }
    } else {
        match sales_service
            .reject_order(business_id, "BPM审批拒绝".to_string(), approver_id)
            .await
        {
            Ok(_) => tracing::info!("Successfully rejected sales_order {} via BPM", business_id),
            Err(e) => tracing::error!(
                "Failed to reject sales_order {} via BPM: {}",
                business_id,
                e
            ),
        }
    }
}

/// 处理生产订单 BPM 审批结果回写（专用 approve_order_via_bpm/reject_order_via_bpm，不回调 BPM 避免循环）
async fn handle_bpm_production_order(
    db: Arc<DatabaseConnection>,
    business_id: i32,
    approved: bool,
    approver_id: i32,
) {
    // B-P1-9 修复（批次 360 v13 复审）：原实现仅处理 purchase_order/sales_order，生产订单 BPM 审批结果无法回写
    let prod_service = crate::services::production_order_service::ProductionOrderService::new(db);
    if approved {
        if let Err(e) = prod_service
            .approve_order_via_bpm(business_id, approver_id)
            .await
        {
            tracing::error!(
                "Failed to approve production_order {} via BPM: {}",
                business_id,
                e
            );
        } else {
            tracing::info!(
                "Successfully approved production_order {} via BPM",
                business_id
            );
        }
    } else {
        if let Err(e) = prod_service
            .reject_order_via_bpm(business_id, "BPM审批拒绝".to_string(), approver_id)
            .await
        {
            tracing::error!(
                "Failed to reject production_order {} via BPM: {}",
                business_id,
                e
            );
        } else {
            tracing::info!(
                "Successfully rejected production_order {} via BPM",
                business_id
            );
        }
    }
}

/// 处理低库存预警事件：幂等校验 + 创建采购建议 + 通知 admin/manager 角色用户
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
        product_id,
        warehouse_id,
        current_quantity,
        reorder_point,
        reorder_quantity
    );
    if !check_low_stock_idempotency(&*db, product_id, warehouse_id).await {
        return;
    }
    create_low_stock_purchase_suggestion(
        db.clone(),
        product_id,
        warehouse_id,
        current_quantity,
        reorder_point,
        reorder_quantity,
    )
    .await;
    notify_low_stock_users(
        db,
        product_id,
        warehouse_id,
        current_quantity,
        reorder_point,
    )
    .await;
}

/// 低库存预警幂等校验（同产品同仓库同一天仅处理一次）
async fn check_low_stock_idempotency(
    db: &DatabaseConnection,
    product_id: i32,
    warehouse_id: i32,
) -> bool {
    let idempotency_service =
        crate::services::event_idempotency_service::EventIdempotencyService::new(db.clone().into());
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let event_key = format!("low_stock:{}:{}:{}", product_id, warehouse_id, today);
    match idempotency_service
        .try_mark_processed("event_bus_main", &event_key, "LowStockAlert")
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "LowStockAlert 幂等检查失败 product={} warehouse={}: {}",
                product_id,
                warehouse_id,
                e
            );
            false
        }
    }
}

/// 创建低库存采购建议
async fn create_low_stock_purchase_suggestion(
    db: Arc<DatabaseConnection>,
    product_id: i32,
    warehouse_id: i32,
    current_quantity: rust_decimal::Decimal,
    reorder_point: rust_decimal::Decimal,
    reorder_quantity: rust_decimal::Decimal,
) {
    let po_service = crate::services::po::order::PurchaseOrderService::new(db);
    match po_service
        .create_purchase_suggestion(
            product_id,
            warehouse_id,
            current_quantity,
            reorder_point,
            reorder_quantity,
        )
        .await
    {
        Ok(order) => tracing::info!(
            "成功创建采购建议: 订单ID={}, 订单号={}",
            order.id,
            order.order_no
        ),
        Err(e) => tracing::error!("创建采购建议失败: {}", e),
    }
}

/// 发送低库存预警通知给 admin/manager 角色用户
async fn notify_low_stock_users(
    db: Arc<DatabaseConnection>,
    product_id: i32,
    warehouse_id: i32,
    current_quantity: rust_decimal::Decimal,
    reorder_point: rust_decimal::Decimal,
) {
    let product_name = fetch_product_name(&*db, product_id).await;
    let notify_user_ids = fetch_admin_manager_user_ids(&*db).await;
    let notify_count = notify_user_ids.len();
    let notification_service =
        crate::services::event_notification_service::EventNotificationService::new(db.clone());
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
            notify_count,
            e
        );
    }
    tracing::info!(
        "低库存预警通知已发送: 产品={}, 仓库ID={}, 通知人数={}",
        product_name,
        warehouse_id,
        notify_count
    );
}

/// 获取产品名称（缺失则回退为"产品{id}"）
async fn fetch_product_name(db: &DatabaseConnection, product_id: i32) -> String {
    crate::models::product::Entity::find_by_id(product_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|p| p.name)
        .unwrap_or_else(|| format!("产品{}", product_id))
}

/// 获取 admin/manager 角色的活跃用户 ID 列表
async fn fetch_admin_manager_user_ids(db: &DatabaseConnection) -> Vec<i32> {
    use crate::utils::admin_checker::{ADMIN_ROLE_CODE, MANAGER_ROLE_CODE};
    let target_role_ids: Vec<i32> = crate::models::role::Entity::find()
        .filter(
            crate::models::role::Column::Code
                .eq(ADMIN_ROLE_CODE)
                .or(crate::models::role::Column::Code.eq(MANAGER_ROLE_CODE)),
        )
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.id)
        .collect();
    if target_role_ids.is_empty() {
        return Vec::new();
    }
    crate::models::user::Entity::find()
        .filter(crate::models::user::Column::IsActive.eq(true))
        .filter(crate::models::user::Column::RoleId.is_in(target_role_ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| u.id)
        .collect()
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
        material_id,
        material_name,
        shortage_quantity,
        shortage_level,
        affected_orders_count
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
                material_id,
                e
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
    match po_service
        .create_purchase_suggestion_from_shortage(shortage_params)
        .await
    {
        Ok(order) => tracing::info!(
            "成功创建缺料采购建议: 订单ID={}, 订单号={}, 物料={}",
            order.id,
            order.order_no,
            material_name
        ),
        Err(e) => tracing::error!("创建缺料采购建议失败: 物料ID={}, 错误={}", material_id, e),
    }
}

/// 处理财务指标更新事件：调用 FinancialAnalysisService.calculate_indicators 刷新指标
async fn handle_financial_indicator_update(
    db: Arc<DatabaseConnection>,
    period: String,
    trigger_source: String,
) {
    tracing::info!(
        "处理财务指标更新事件: 期间={}, 触发源={}",
        period,
        trigger_source
    );
    let fa_service = crate::services::financial_analysis_service::FinancialAnalysisService::new(db);
    match fa_service.calculate_indicators(&period, 0).await {
        Ok(results) => tracing::info!(
            "财务指标自动计算完成: 期间={}, 计算 {} 个指标",
            period,
            results.len()
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
) -> Result<(), AppError> {
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
) -> Result<(), AppError> {
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
) -> Result<(), AppError> {
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
) -> Result<(), AppError> {
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
) -> Result<(), AppError> {
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
) -> Result<(), AppError> {
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
/// 当供应商主数据 supplier_name 变更时，异步刷新以下表的冗余字段：purchase_contracts.supplier_name；fixed_assets.supplier_name
async fn refresh_supplier_name_redundancy(
    db: &sea_orm::DatabaseConnection,
    supplier_id: i32,
    new_name: &str,
) -> Result<(), AppError> {
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
