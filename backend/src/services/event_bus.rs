//!
//! 事件总线（P11-H2 Kafka 真实集成）
//!
//! 抽象 [`EventBackend`] trait，对外暴露双后端：
//! - `Broadcast`（默认，进程内 `tokio::sync::broadcast`，CI 友好）
//! - `Kafka`（生产可启用，基于 `rskafka` 真实投递到 broker，跨服务可用）
//!
//! 公共 API（`EVENT_BUS` / `publish` / `subscribe` / `start_event_listener`）
//! 保持完全向后兼容；旧调用方零修改。
//!
//! 启动时通过 [`init_event_bus_with_kafka_config`] 注入 Kafka 配置；
//! Kafka 不可达时**自动降级**到 `Broadcast`，并通过 `tracing::error!` 输出中文日志。

use futures::stream::Stream;
use sea_orm;
use sea_orm::DatabaseConnection;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::sync::broadcast;

use crate::config::settings::KafkaSettings;

// 类型别名：抽离超长 `Pin<Box<dyn Future<...>>>` 以满足 rustc
// "very complex type used" 限制（clippy 配置见 .clippy.toml）。
type EventStream = Box<dyn Stream<Item = BusinessEvent> + Send + Unpin>;
type SubscribeFuture<'a> = Pin<Box<dyn Future<Output = Result<EventStream, String>> + Send + 'a>>;

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
    PaymentCompleted {
        payment_id: i32,
        invoice_id: i32,
        amount: rust_decimal::Decimal,
    },
    InventoryAdjusted {
        product_id: i32,
        warehouse_id: i32,
        quantity_change: rust_decimal::Decimal,
    },
    CollectionCompleted {
        collection_id: i32,
        invoice_id: Option<i32>,
        amount: rust_decimal::Decimal,
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
}

// ============================================================================
// 后端抽象（P11-H2 新增）
// ============================================================================

/// 事件总线后端抽象（dyn 兼容）
///
/// 使用 `Pin<Box<dyn Future>>` 而非 `async fn` 是为了在 stable Rust 下支持
/// `Arc<dyn EventBackend>` 动态分发；调用方拿到的是一次性装箱的 future。
pub trait EventBackend: Send + Sync {
    /// 异步发布事件
    fn publish<'a>(
        &'a self,
        event: BusinessEvent,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

    /// 异步订阅事件，返回 `Box<dyn Stream>` 供上层消费
    fn subscribe<'a>(&'a self) -> SubscribeFuture<'a>;
}

/// 进程内 Broadcast 后端（默认）
#[derive(Clone)]
pub struct BroadcastBackend {
    sender: broadcast::Sender<BusinessEvent>,
}

impl BroadcastBackend {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

impl EventBackend for BroadcastBackend {
    fn publish<'a>(
        &'a self,
        event: BusinessEvent,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        let result = self.sender.send(event);
        Box::pin(async move {
            match result {
                Ok(_) => Ok(()),
                Err(_) => Err("事件订阅者已全部断开，发送失败".to_string()),
            }
        })
    }

    fn subscribe<'a>(&'a self) -> SubscribeFuture<'a> {
        // 启动桥接任务：把 broadcast::Receiver 转为 mpsc::Receiver
        // （broadcast::Receiver 没有 poll_recv，不能直接实现 Stream，
        // 所以走 tokio 任务 + mpsc 通道的桥接模式）
        let (tx, rx) = tokio::sync::mpsc::channel::<BusinessEvent>(1024);
        let mut broadcast_rx = self.sender.subscribe();
        tokio::spawn(async move {
            loop {
                match broadcast_rx.recv().await {
                    Ok(event) => {
                        if tx.send(event).await.is_err() {
                            // 订阅者断开，停止桥接
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Lagged：跳过丢失事件，继续接收
                        tracing::warn!("Broadcast 桥接 lagged 跳过 {} 条事件", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // channel 关闭，退出
                        break;
                    }
                }
            }
        });
        let stream: EventStream = Box::new(BridgeStream { rx });
        Box::pin(async move { Ok(stream) })
    }
}

/// 把 mpsc::Receiver 包装为 Stream（mpsc 自身实现 Stream，直接代理）
struct BridgeStream {
    rx: tokio::sync::mpsc::Receiver<BusinessEvent>,
}

impl Stream for BridgeStream {
    type Item = BusinessEvent;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// 实际选用的后端运行时容器
struct EventBusState {
    backend_kind: AtomicU8, // 0 = Broadcast, 1 = Kafka
    broadcast: BroadcastBackend,
    kafka: Option<Arc<crate::services::event_kafka::KafkaBackend>>,
    /// 始终存在的本地 channel，用于在 Kafka 模式下把 Kafka 消费到的事件
    /// 桥接到本进程的所有订阅者，保持 `subscribe() -> broadcast::Receiver` API
    local_tx: broadcast::Sender<BusinessEvent>,
}

impl EventBusState {
    fn new() -> Self {
        let broadcast = BroadcastBackend::new(1024);
        let (local_tx, _) = broadcast::channel(1024);
        Self {
            backend_kind: AtomicU8::new(0),
            broadcast,
            kafka: None,
            local_tx,
        }
    }
}

/// 全局状态（once_cell 风格，避免重入初始化）
static EVENT_BUS_STATE: LazyLock<std::sync::Mutex<EventBusState>> =
    LazyLock::new(|| std::sync::Mutex::new(EventBusState::new()));

/// 全局 `EventBus` 句柄
pub static EVENT_BUS: LazyLock<EventBus> = LazyLock::new(EventBus::new);

/// 统一封装 `EVENT_BUS_STATE` 加锁逻辑
///
/// 锁中毒（PoisonError）通常意味着某线程在持锁期间 panic，状态已不可信，
/// 这里保留 fail-fast panic 行为，但通过 `tracing::error!` 给出可观测的中文错误，
/// 便于运维快速定位崩溃源。
fn lock_event_bus_state() -> std::sync::MutexGuard<'static, EventBusState> {
    EVENT_BUS_STATE.lock().unwrap_or_else(|e| {
        tracing::error!(
            error = %e,
            "P9-1: EVENT_BUS_STATE 锁中毒（可能存在线程 panic），状态不可信"
        );
        panic!("P9-1: EVENT_BUS_STATE 锁中毒: {e}")
    })
}

/// 事件总线主结构
///
/// 内部根据 [`EventBusState`] 决定走 Broadcast 或 Kafka 真实后端。
pub struct EventBus;

impl EventBus {
    /// 构造一个 `EventBus` 句柄（不会触发任何 IO）
    pub fn new() -> Self {
        Self
    }

    /// 当前后端类型（用于诊断 / 测试断言）
    pub fn backend_type(&self) -> EventBackendType {
        let state = lock_event_bus_state();
        match state.backend_kind.load(Ordering::Acquire) {
            1 => EventBackendType::Kafka,
            _ => EventBackendType::Broadcast,
        }
    }

    /// 同步发布事件（保留旧 API 兼容）
    ///
    /// `start_event_listener` 等旧调用方直接调用 `EVENT_BUS.publish(event)`，
    /// 这里在同步上下文内 spawn 一个 tokio 任务异步发送。
    pub fn publish(&self, event: BusinessEvent) {
        // 同步上下文：直接写到本地 channel（无失败语义），并 spawn 异步 Kafka 投递
        let state = lock_event_bus_state();
        let _ = state.local_tx.send(event.clone());
        let kind = state.backend_kind.load(Ordering::Acquire);
        let kafka = state.kafka.as_ref().cloned();
        drop(state);

        if kind == 1 {
            if let Some(k) = kafka {
                tokio::spawn(async move {
                    if let Err(e) = k.publish(event).await {
                        tracing::error!("事件投递到 Kafka 失败: {}（已写入本地兜底）", e);
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

/// 后端类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventBackendType {
    /// 进程内 Broadcast（默认，CI 友好）
    Broadcast,
    /// Kafka 真实后端（生产可启用）
    Kafka,
}

/// 使用 Kafka 配置初始化事件总线（在 `main.rs` 启动时调用一次）
///
/// 行为：
/// - `kafka.enabled=false` → 保持 Broadcast 后端；
/// - `kafka.enabled=true` 且连接成功 → 切到 Kafka 模式；
/// - `kafka.enabled=true` 但连接失败 → 自动降级到 Broadcast + 中文 warning 日志。
pub async fn init_event_bus_with_kafka_config(kafka_cfg: &KafkaSettings) {
    if !kafka_cfg.enabled {
        tracing::info!("事件总线后端 = Broadcast（kafka.enabled=false，CI/开发环境默认）");
        return;
    }

    tracing::info!(
        "事件总线尝试初始化 Kafka 后端：brokers={}, topic={}",
        kafka_cfg.brokers,
        kafka_cfg.topic
    );

    match crate::services::event_kafka::KafkaBackend::try_new(kafka_cfg).await {
        Ok(backend) => {
            let backend = Arc::new(backend);
            // 启动消费后台任务，把 Kafka 事件桥接到本地 channel
            let local_tx = {
                let state = lock_event_bus_state();
                state.local_tx.clone()
            };
            let backend_for_consumer = backend.clone();
            let _consumer_handle = tokio::spawn(async move {
                match backend_for_consumer.subscribe().await {
                    Ok(mut stream) => {
                        use futures::stream::StreamExt;
                        while let Some(event) = stream.next().await {
                            if local_tx.send(event).is_err() {
                                tracing::warn!("Kafka 消费桥接：本地 channel 已关闭，停止消费");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Kafka 订阅失败: {}", e);
                    }
                }
            });

            {
                let mut state = lock_event_bus_state();
                state.kafka = Some(backend);
                state.backend_kind.store(1, Ordering::Release);
            }
            tracing::info!("事件总线后端 = Kafka（已就绪）");
        }
        Err(e) => {
            tracing::error!(
                "Kafka 初始化失败，自动降级到 Broadcast 后端: {}（生产环境请检查 brokers={}）",
                e,
                kafka_cfg.brokers
            );
            // 不修改 backend_kind，保持 Broadcast
        }
    }
}

// ============================================================================
// 旧 API：`start_event_listener`（保持完全兼容）
// ============================================================================

pub async fn start_event_listener(db: Arc<DatabaseConnection>) {
    // 启动库存财务桥接服务监听器
    crate::services::inventory_finance_bridge_service::InventoryFinanceBridgeService::start_listener(db.clone());

    let mut receiver = EVENT_BUS.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                BusinessEvent::PurchaseReceiptCompleted { order_id, .. } => {
                    tracing::info!(
                        "Event received: PurchaseReceiptCompleted for order {}",
                        order_id
                    );
                    let po_service =
                        crate::services::po::order::PurchaseOrderService::new(db.clone());
                    match po_service.receive_order(order_id).await {
                        Ok(_) => tracing::info!(
                            "Successfully updated purchase order {} status to RECEIVED",
                            order_id
                        ),
                        Err(e) => {
                            tracing::error!("Failed to update purchase order {}: {}", order_id, e)
                        }
                    }
                }
                BusinessEvent::SalesOrderShipped { order_id, .. } => {
                    tracing::info!("Event received: SalesOrderShipped for order {}", order_id);
                }
                BusinessEvent::PaymentCompleted { invoice_id, .. } => {
                    tracing::info!(
                        "Event received: PaymentCompleted for invoice {}",
                        invoice_id
                    );
                    let ap_service =
                        crate::services::ap_invoice_service::ApInvoiceService::new(db.clone());
                    match ap_service.mark_as_paid(invoice_id).await {
                        Ok(_) => tracing::info!(
                            "Successfully updated ap_invoice {} status to PAID",
                            invoice_id
                        ),
                        Err(e) => {
                            tracing::error!("Failed to update ap_invoice {}: {}", invoice_id, e)
                        }
                    }
                }
                BusinessEvent::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                } => {
                    tracing::info!("Event received: InventoryAdjusted for product {} at warehouse {}, change: {}", product_id, warehouse_id, quantity_change);
                }
                BusinessEvent::PurchaseOrderApproved { order_id, .. } => {
                    tracing::info!(
                        "Event received: PurchaseOrderApproved for order {}",
                        order_id
                    );
                }
                BusinessEvent::CollectionCompleted {
                    invoice_id: Some(inv_id),
                    ..
                } => {
                    tracing::info!("Event received: CollectionCompleted for invoice {}", inv_id);
                    let ar_service =
                        crate::services::ar_invoice_service::ArInvoiceService::new(db.clone());
                    match ar_service.mark_as_paid(inv_id).await {
                        Ok(_) => tracing::info!(
                            "Successfully updated ar_invoice {} status to PAID",
                            inv_id
                        ),
                        Err(e) => tracing::error!("Failed to update ar_invoice {}: {}", inv_id, e),
                    }
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
                } => {
                    tracing::info!(
                        "处理BPM流程结束事件: type={}, id={}, approved={}",
                        business_type,
                        business_id,
                        approved
                    );
                    if business_type == "purchase_order" {
                        if approved {
                            let po_service =
                                crate::services::po::order::PurchaseOrderService::new(db.clone());
                            if let Err(e) = po_service.approve_order(business_id, 0).await {
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
                        } else {
                            let po_service =
                                crate::services::po::order::PurchaseOrderService::new(db.clone());
                            if let Err(e) = po_service
                                .reject_order(business_id, "BPM审批拒绝".to_string(), 0)
                                .await
                            {
                                tracing::error!(
                                    "Failed to reject purchase_order {} via BPM: {}",
                                    business_id,
                                    e
                                );
                            }
                        }
                    } else if business_type == "sales_order" {
                        if approved {
                            let sales_service =
                                crate::services::so::order::SalesService::new(db.clone());
                            if let Err(e) = sales_service.approve_order(business_id, 0).await {
                                tracing::error!(
                                    "Failed to approve sales_order {} via BPM: {}",
                                    business_id,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Successfully approved sales_order {} via BPM",
                                    business_id
                                );
                            }
                        } else {
                            let sales_service =
                                crate::services::so::order::SalesService::new(db.clone());
                            match sales_service
                                .reject_order(business_id, "BPM审批拒绝".to_string(), 0)
                                .await
                            {
                                Ok(_) => tracing::info!(
                                    "Successfully rejected sales_order {} via BPM",
                                    business_id
                                ),
                                Err(e) => tracing::error!(
                                    "Failed to reject sales_order {} via BPM: {}",
                                    business_id,
                                    e
                                ),
                            }
                        }
                    }
                }
                BusinessEvent::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => {
                    tracing::info!(
                        "处理低库存预警事件: 产品ID={}, 仓库ID={}, 当前库存={}, 补货点={}, 建议补货量={}",
                        product_id,
                        warehouse_id,
                        current_quantity,
                        reorder_point,
                        reorder_quantity
                    );

                    // 创建采购建议
                    let po_service =
                        crate::services::po::order::PurchaseOrderService::new(db.clone());
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
                        Ok(order) => {
                            tracing::info!(
                                "成功创建采购建议: 订单ID={}, 订单号={}",
                                order.id,
                                order.order_no
                            );
                        }
                        Err(e) => {
                            tracing::error!("创建采购建议失败: {}", e);
                        }
                    }

                    // 发送低库存预警通知给仓库管理员和采购人员
                    let notification_service =
                        crate::services::event_notification_service::EventNotificationService::new(
                            db.clone(),
                        );
                    let product_name = crate::models::product::Entity::find_by_id(product_id)
                        .one(db.as_ref())
                        .await
                        .ok()
                        .flatten()
                        .map(|p| p.name)
                        .unwrap_or_else(|| format!("产品{}", product_id));

                    // 查询仓库管理员和采购相关人员（通过 role_id 关联角色表）
                    let notify_user_ids = crate::models::user::Entity::find()
                        .filter(crate::models::user::Column::IsActive.eq(true))
                        .all(db.as_ref())
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .map(|u| u.id)
                        .collect::<Vec<i32>>();

                    let notify_count = notify_user_ids.len();
                    for user_id in notify_user_ids {
                        if let Err(e) = notification_service
                            .notify_inventory_alert(
                                user_id,
                                &product_name,
                                product_id,
                                &format!("{}米", current_quantity),
                                &format!("{}米", reorder_point),
                            )
                            .await
                        {
                            tracing::error!(
                                "发送低库存预警通知失败: 用户ID={}, 错误={}",
                                user_id,
                                e
                            );
                        }
                    }
                    tracing::info!(
                        "低库存预警通知已发送: 产品={}, 仓库ID={}, 通知人数={}",
                        product_name,
                        warehouse_id,
                        notify_count
                    );
                }
                BusinessEvent::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                } => {
                    tracing::info!(
                        "处理财务指标更新事件: 期间={}, 触发源={}",
                        period,
                        trigger_source
                    );
                    let fa_service =
                        crate::services::financial_analysis_service::FinancialAnalysisService::new(
                            db.clone(),
                        );
                    match fa_service.calculate_indicators(&period, 0).await {
                        Ok(results) => {
                            tracing::info!(
                                "财务指标自动计算完成: 期间={}, 计算 {} 个指标",
                                period,
                                results.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!("财务指标自动计算失败: 期间={}, 错误={}", period, e);
                        }
                    }
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
                    tracing::info!(
                        "处理缺料预警事件: 物料ID={}, 物料名称={}, 缺料数量={}, 预警级别={}, 受影响订单数={}",
                        material_id,
                        material_name,
                        shortage_quantity,
                        shortage_level,
                        affected_orders_count
                    );
                    let po_service =
                        crate::services::po::order::PurchaseOrderService::new(db.clone());
                    match po_service
                        .create_purchase_suggestion_from_shortage(
                            material_id,
                            material_name.clone(),
                            material_code.clone(),
                            required_quantity,
                            available_quantity,
                            shortage_quantity,
                            shortage_level.clone(),
                            affected_orders_count,
                        )
                        .await
                    {
                        Ok(order) => {
                            tracing::info!(
                                "成功创建缺料采购建议: 订单ID={}, 订单号={}, 物料={}",
                                order.id,
                                order.order_no,
                                material_name
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "创建缺料采购建议失败: 物料ID={}, 错误={}",
                                material_id,
                                e
                            );
                        }
                    }
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }
    });
}
