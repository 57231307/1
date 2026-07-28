//!
//! 事件总线（P11-H2 Kafka 真实集成）— facade
//!
//! 双后端实现：
//! - `Broadcast`（默认，进程内 `tokio::sync::broadcast`，CI 友好）
//! - `Kafka`（生产可启用，基于 `rskafka` 真实投递到 broker，跨服务可用）
//!
//! 公共 API（`EVENT_BUS` / `publish` / `subscribe` / `start_event_listener` /
//! `init_event_bus_with_kafka_config` / `shutdown_event_bus`）保持完全向后兼容；
//! 旧调用方零修改。
//!
//! 本文件为 facade：仅保留公共类型定义（`ShippedItem` / `BusinessEvent` /
//! `EventBusState` / `EventBus`）、全局静态状态（`EVENT_BUS` / `EVENT_BUS_STATE` /
//! `MAIN_LISTENER_HANDLE`）、`new` 构造器、`publish`/`subscribe` 公共 API、
//! `Default` 实现，以及 `lock_event_bus_state` 状态访问 helper。
//!
//! 业务实现已拆分到 [`crate::services::event_bus_ops`] 子模块：
//! - `kafka`：`init_event_bus_with_kafka_config` + Kafka 消费桥接
//! - `listener`：`start_event_listener` + `shutdown_event_bus` + 各事件 arm 处理 helper
//!
//! 启动时通过 [`init_event_bus_with_kafka_config`] 注入 Kafka 配置；
//! Kafka 不可达时**自动降级**到 `Broadcast`，并通过 `tracing::error!` 输出中文日志。
//!
//! 批次 120 P2-10 修复：删除未接入业务的 `EventBackend` trait、`BroadcastBackend`、
//! `BridgeStream`、`EventBackendType` 枚举、`backend_type()` 方法。
//! 原因：`KafkaBackend` 已绕过 trait 抽象走独立路径（`EventBusState.kafka` 字段
//! 直接持有 `Arc<KafkaBackend>`），`BroadcastBackend` 从未被 `EVENT_BUS.publish`
//! 或 `subscribe` 调用（这俩方法直接操作 `local_tx: broadcast::Sender`），
//! trait 与 BroadcastBackend 与 BridgeStream 与类型别名全部为零业务调用方的死代码。

use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::sync::broadcast;

// 事件总线业务实现子模块（kafka 初始化 / 事件监听），由父模块 services 声明。
// facade 对外 re-export 三个自由函数，保持 `crate::services::event_bus::*` 旧路径兼容。
pub use crate::services::event_bus_ops::{
    init_event_bus_with_kafka_config, shutdown_event_bus, start_event_listener,
};

// ============================================================================
// 公共类型（业务事件枚举）
// ============================================================================

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ShippedItem {
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
}

#[derive(Clone, Debug)]
pub enum BusinessEvent {
    PurchaseReceiptCompleted {
        receipt_id: i32,
        order_id: i32,
        supplier_id: i32,
    },
    SalesOrderShipped {
        order_id: i32,
        customer_id: i32,
        items: Vec<ShippedItem>,
    },
    // B-P1-4 修复（批次 361 v13 复审）：销售订单状态变更事件
    // 原实现仅 ship_order 发布 SalesOrderShipped，其余 5 个状态变更方法无事件，
    // 导致下游订阅方（财务指标刷新、BI 报表等）无法感知订单状态变化。
    SalesOrderSubmitted {
        order_id: i32,
        customer_id: i32,
        user_id: i32,
    },
    SalesOrderApproved {
        order_id: i32,
        customer_id: i32,
        user_id: i32,
    },
    SalesOrderCompleted {
        order_id: i32,
        customer_id: i32,
        user_id: i32,
    },
    SalesOrderCancelled {
        order_id: i32,
        customer_id: i32,
        user_id: i32,
    },
    SalesOrderRejected {
        order_id: i32,
        customer_id: i32,
        user_id: i32,
    },
    PaymentCompleted {
        payment_id: i32,
        invoice_id: i32,
        amount: rust_decimal::Decimal,
        /// 批次 97 P1-2 修复：付款操作人 ID
        /// 用于 mark_as_paid 审计日志透传，替代原硬编码 Some(0)
        user_id: i32,
    },
    CollectionCompleted {
        collection_id: i32,
        invoice_id: Option<i32>,
        amount: rust_decimal::Decimal,
        /// P1 1-1 修复（批次 78 v1 复审）：收款操作人 ID
        /// 用于 mark_as_paid 审计日志透传，替代原硬编码 Some(0)
        user_id: i32,
    },
    PurchaseOrderApproved {
        order_id: i32,
        supplier_id: i32,
    },
    InventoryCountCompleted {
        count_id: i32,
        variance_count: i32,
    },
    BpmProcessFinished {
        business_type: String,
        business_id: i32,
        approved: bool,
        /// P2 5-18 修复：审批人 ID（从 BPM 事件 payload 携带，替代原硬编码 0）
        approver_id: i32,
    },
    LowStockAlert {
        product_id: i32,
        warehouse_id: i32,
        current_quantity: rust_decimal::Decimal,
        reorder_point: rust_decimal::Decimal,
        reorder_quantity: rust_decimal::Decimal,
    },
    FinancialIndicatorUpdate {
        period: String,
        trigger_source: String,
    },
    MaterialShortageAlert {
        material_id: i32,
        material_name: String,
        material_code: String,
        required_quantity: rust_decimal::Decimal,
        available_quantity: rust_decimal::Decimal,
        shortage_quantity: rust_decimal::Decimal,
        shortage_level: String,
        affected_orders_count: i32,
    },
    InventoryTransactionCreated {
        transaction_id: i32,
        transaction_type: String,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: rust_decimal::Decimal,
        quantity_kg: rust_decimal::Decimal,
        source_bill_type: Option<String>,
        source_bill_no: Option<String>,
        source_bill_id: Option<i32>,
        batch_no: String,
        color_no: String,
        created_by: Option<i32>,
    },
    // B-P1-3 修复（批次 384 v13 复审）：客户/供应商主数据变更事件
    // 原实现 update_customer/update_supplier 不发布事件，下游关联单据冗余字段无法同步刷新，
    // 导致 AR/AP/合同等单据的 customer_name/supplier_name 与主数据不一致。
    CustomerUpdated {
        customer_id: i32,
        customer_name: String,
        user_id: i32,
    },
    SupplierUpdated {
        supplier_id: i32,
        supplier_name: String,
        user_id: i32,
    },
    // v14 批次 420 修复 T-P1-3：染色完成事件
    // 原实现 complete_dye_batch 仅做状态更新，未发布任何业务事件，
    // 导致下游（质检单生成、染缸产能统计、成本结转、BI 生产报表）无法被动感知。
    DyeBatchCompleted {
        batch_id: i32,
        batch_no: String,
        color_no: Option<String>,
        greige_fabric_id: Option<i32>,
        planned_quantity: Option<rust_decimal::Decimal>,
        completed_by: Option<i32>,
    },
    // v14 批次 420 修复 T-P1-3：质检完成事件
    // 用于贯通质检完成→库存入库/成本结转的业务链路。
    QualityInspectionCompleted {
        inspection_id: i32,
        batch_id: Option<i32>,
        product_id: i32,
        /// 质检结果（passed/failed/conditional）
        result: String,
        inspector_id: Option<i32>,
    },
    // V15 Batch05-P1-3：面料行业 6 个核心业务事件
    // 染整工序扫码上报事件（工序进度实时感知）
    ProcessStepReported {
        step_record_id: i32,
        flow_card_id: i32,
        route_code: String,
        operator_id: Option<i32>,
        started_at: Option<chrono::DateTime<chrono::Utc>>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
        quantity: Option<rust_decimal::Decimal>,
    },
    // 缸号状态变更事件（设备占用/释放、看板更新）
    DyeBatchStatusChanged {
        batch_id: i32,
        batch_no: String,
        from_status: String,
        to_status: String,
        transition_code: String,
        operator_id: Option<i32>,
        transition_at: chrono::DateTime<chrono::Utc>,
    },
    // 验布分级事件（A/B/C 级流向：入库/降级/返工）
    FabricInspectionGraded {
        inspection_id: i32,
        batch_id: Option<i32>,
        grade: String,
        handling_method: Option<String>,
        inspector_id: Option<i32>,
    },
    // 产量上报事件（工资计算/成本归集被动触发）
    ProductionQuantityReported {
        step_record_id: i32,
        flow_card_id: i32,
        operator_id: Option<i32>,
        actual_quantity: rust_decimal::Decimal,
        qualified_quantity: rust_decimal::Decimal,
    },
    // 能耗采集事件（能耗异常告警/月末分摊被动触发）
    EnergyConsumptionRecorded {
        record_id: i32,
        workshop: Option<String>,
        meter_type: String,
        consumption: rust_decimal::Decimal,
        cost: rust_decimal::Decimal,
        recorded_at: chrono::DateTime<chrono::Utc>,
    },
    // 色卡发放事件（色卡库存扣减/过期回收/客户对色反馈）
    ColorCardIssued {
        issue_id: i32,
        color_card_id: i32,
        customer_id: Option<i32>,
        issued_by: Option<i32>,
        issued_at: chrono::DateTime<chrono::Utc>,
    },
    // V15 Batch04-P1-3：工资确认/发放事件（成本归集被动触发）
    WageConfirmed {
        wage_record_id: i32,
        record_no: String,
        total_amount: rust_decimal::Decimal,
        confirmed_by: i32,
    },
    WagePaid {
        wage_record_id: i32,
        record_no: String,
        total_amount: rust_decimal::Decimal,
        paid_by: i32,
    },
    // V15 Batch04-P1-5：委外加工业务事件（发料/加工/结算/完成）
    OutsourcingMaterialIssued {
        order_id: i32,
        order_no: String,
        order_type: String,
        supplier_id: i32,
        issue_quantity: rust_decimal::Decimal,
        voucher_no_issue: Option<String>,
    },
    OutsourcingProcessingRecorded {
        order_id: i32,
        order_no: String,
        order_type: String,
        supplier_id: i32,
    },
    OutsourcingOrderSettled {
        order_id: i32,
        order_no: String,
        order_type: String,
        supplier_id: i32,
        processing_fee: rust_decimal::Decimal,
        freight_fee: rust_decimal::Decimal,
        normal_loss: rust_decimal::Decimal,
        abnormal_loss: rust_decimal::Decimal,
        total_cost: rust_decimal::Decimal,
        unit_cost: rust_decimal::Decimal,
        voucher_no_fee: Option<String>,
    },
    OutsourcingOrderCompleted {
        order_id: i32,
        order_no: String,
        order_type: String,
        supplier_id: i32,
        return_quantity: rust_decimal::Decimal,
        voucher_no_receipt: Option<String>,
    },
    // V15 Batch04-P1-6：业务模式切换事件
    BusinessModeChanged {
        mode_id: i32,
        mode_code: String,
        mode_name: String,
        changed_by: i32,
    },
    OrderBusinessModeLinked {
        document_type: String,
        document_id: i32,
        document_no: String,
        mode_id: i32,
        mode_code: String,
        mode_name: String,
    },
}

// ============================================================================
// 后端运行时容器
// ============================================================================
//
// 批次 120 P2-10 修复：删除 EventBackend trait + BroadcastBackend + BridgeStream
// + EventStream/SubscribeFuture 类型别名。原因：
// - KafkaBackend 已绕过 trait 抽象走独立路径（EventBusState.kafka 字段直接持有
//   `Arc<KafkaBackend>`，publish 时通过 `k.publish(event).await` 调用）
// - BroadcastBackend 从未被 EVENT_BUS.publish/subscribe 调用（这俩方法直接操作
//   `local_tx: broadcast::Sender`）
// - trait + BroadcastBackend + BridgeStream + 类型别名全部为零业务调用方的死代码
//
// 保留的双后端切换逻辑：
// - `backend_kind == 0`：仅本地 broadcast::Sender（默认，CI/开发环境）
// - `backend_kind == 1`：本地 broadcast::Sender + Kafka 投递（生产环境）
// - `local_tx` 始终存在，Kafka 模式下用于把 Kafka 消费到的事件桥接到本进程订阅者

/// 实际选用的后端运行时容器（`pub(crate)` 字段供 event_bus_ops 子模块读写）
pub(crate) struct EventBusState {
    pub(crate) backend_kind: AtomicU8, // 0 = Broadcast, 1 = Kafka
    pub(crate) kafka: Option<Arc<crate::services::event_kafka::KafkaBackend>>,
    /// 本地 channel，Kafka 模式下桥接事件到本进程订阅者
    pub(crate) local_tx: broadcast::Sender<BusinessEvent>,
    /// Kafka 消费桥接 spawn 句柄（shutdown 时 abort，避免泄漏）
    pub(crate) consumer_handle: Option<tokio::task::JoinHandle<()>>,
}

impl EventBusState {
    fn new() -> Self {
        let (local_tx, _) = broadcast::channel(1024);
        Self {
            backend_kind: AtomicU8::new(0),
            kafka: None,
            local_tx,
            consumer_handle: None,
        }
    }
}

/// 全局状态（once_cell 风格，避免重入初始化）
static EVENT_BUS_STATE: LazyLock<std::sync::Mutex<EventBusState>> =
    LazyLock::new(|| std::sync::Mutex::new(EventBusState::new()));

/// 全局 `EventBus` 句柄
pub static EVENT_BUS: LazyLock<EventBus> = LazyLock::new(EventBus::new);

/// 统一封装 `EVENT_BUS_STATE` 加锁逻辑（锁中毒时优雅降级恢复数据）
pub(crate) fn lock_event_bus_state() -> std::sync::MutexGuard<'static, EventBusState> {
    EVENT_BUS_STATE.lock().unwrap_or_else(|e| {
        tracing::error!(
            error = %e,
            "P9-1: EVENT_BUS_STATE 锁中毒（可能存在线程 panic），恢复数据继续运行以避免服务中断"
        );
        e.into_inner()
    })
}

/// 事件总线主结构（根据 EventBusState 走 Broadcast 或 Kafka 后端）
pub struct EventBus;

impl EventBus {
    /// 构造一个 `EventBus` 句柄（不会触发任何 IO）
    pub fn new() -> Self {
        Self
    }

    /// 同步发布事件（保留旧 API 兼容，同步上下文内 spawn 异步发送）
    pub fn publish(&self, event: BusinessEvent) {
        // 同步上下文：直接写到本地 channel（无失败语义），并 spawn 异步 Kafka 投递
        let state = lock_event_bus_state();
        // L-6 修复（批次 368 v13 复审）：本地 channel 发送失败不再吞错，记录 warn 日志
        //（无订阅者时 send 返回 Err，通常发生在启动初期/关闭末期，不影响业务正确性）
        if state.local_tx.send(event.clone()).is_err() {
            tracing::warn!("事件本地 channel 发送失败：无活跃订阅者（事件将被丢弃）");
        }
        let kind = state.backend_kind.load(Ordering::Acquire);
        let kafka = state.kafka.as_ref().cloned();
        drop(state);

        if kind == 1 {
            if let Some(k) = kafka {
                tokio::spawn(async move {
                    // 批次 8（2026-06-28）：一次性 spawn panic 隔离
                    let result = AssertUnwindSafe(async {
                        if let Err(e) = k.publish(event).await {
                            tracing::error!("事件投递到 Kafka 失败: {}（已写入本地兜底）", e);
                        }
                    })
                    .catch_unwind()
                    .await;
                    if let Err(panic_payload) = result {
                        let panic_msg = panic_payload
                            .downcast_ref::<String>()
                            .map(|s| s.as_str())
                            .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                            .unwrap_or("<非字符串 panic payload>");
                        tracing::error!(
                            panic = %panic_msg,
                            "⚠ Kafka 事件投递 spawn panic 已被隔离（已有本地 channel 兜底）"
                        );
                    }
                });
            }
        }
    }

    /// 订阅事件（返回 `broadcast::Receiver`，旧 API 完全兼容）
    pub fn subscribe(&self) -> broadcast::Receiver<BusinessEvent> {
        let state = lock_event_bus_state();
        state.local_tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// L-28 修复（批次 373 v13 复审）：主事件监听器 spawn 句柄
/// 保存句柄以便 shutdown 时 abort，避免 detached task 泄漏；`pub(crate)`：`event_bus_ops::listener` 的 `start_event_listener` 写入、；`shutdown_event_bus` 读取并 abort。
pub(crate) static MAIN_LISTENER_HANDLE: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(None);
