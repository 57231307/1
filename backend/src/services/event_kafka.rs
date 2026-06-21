//! 事件总线 Kafka 后端实现（P11-H2 高风险任务）
//!
//! 责任：
//! - 通过 `rskafka` 客户端连接 Kafka broker；
//! - 启动时自动创建 topic（失败时容忍：可能已存在）；
//! - `publish` 把 `BusinessEvent` 序列化为 JSON 投递到 Kafka；
//! - `subscribe` 启动后台消费任务并把记录反序列化为 `BusinessEvent` 推送到
//!   `tokio::sync::mpsc` 通道，调用方从流中读取。
//!
//! 设计取舍：
//! - rskafka 0.5 是纯 Rust 实现，无 C/C++ 依赖，CI 不需要 librdkafka；
//! - 启动时连接超时 5s（避免启动卡死）；
//! - 启动失败 → 返回 `Err`，由上层 `event_bus` 自动降级到 `BroadcastBackend`；
//! - publish/subscribe 全部使用 `tracing::error!` 记录中文日志，CI 抓得到。

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::stream::Stream;
use rskafka::client::partition::{Compression, OffsetAt, PartitionClient, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};
use rskafka::record::Record;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::config::settings::KafkaSettings;
use crate::services::event_bus::{BusinessEvent, ShippedItem};

/// Kafka 后端错误类型
///
/// 实际消息以中文描述，便于运维排查；调用方拿到后选择降级还是中断。
#[derive(Debug, Clone)]
pub struct KafkaError(pub String);

impl std::fmt::Display for KafkaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for KafkaError {}

impl From<String> for KafkaError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for KafkaError {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// 业务事件在 Kafka 上的线格式（JSON）
///
/// 与 `BusinessEvent` 字段一一对应，单独建模：
/// - `type` 字段做路由（消费端据此反序列化）；
/// - 其余字段按 variant 填入 `data`。
#[allow(dead_code)] // TODO(tech-debt): 报表/审计模块接入 Kafka 时启用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaEventEnvelope {
    /// 事件类型（与 `BusinessEvent` 变体名一一对应，例如
    /// `PurchaseReceiptCompleted` / `MaterialShortageAlert` 等）
    pub r#type: String,
    /// 事件负载（按 type 反序列化为对应变体字段）
    pub data: serde_json::Value,
    /// 生产端时间戳（RFC3339）
    #[serde(default)]
    pub emitted_at: String,
}

impl KafkaEventEnvelope {
    /// 把 `BusinessEvent` 包装为信封
    ///
    /// 通过 `payload_serde::EventPayload` 中转序列化（不修改原 `BusinessEvent`
    /// 的 `derive` 列表，保持公共 API 最小改动）。
    #[allow(dead_code)] // TODO(tech-debt): 报表/审计模块接入 Kafka 时启用
    pub fn from_event(event: &BusinessEvent) -> Self {
        let r#type = event_type_name(event).to_string();
        let payload = payload_serde::EventPayload::from(event);
        let data = serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null);
        Self {
            r#type,
            data,
            emitted_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 把信封还原为 `BusinessEvent`（失败时返回错误字符串）
    pub fn into_event(self) -> Result<BusinessEvent, String> {
        let payload: payload_serde::EventPayload = serde_json::from_value(self.data)
            .map_err(|e| format!("反序列化 EventPayload 失败: {}", e))?;
        BusinessEvent::try_from(payload)
    }
}

/// 返回 `BusinessEvent` 对应的事件类型字符串（与 `KafkaEventEnvelope.type` 对应）
#[allow(dead_code)] // TODO(tech-debt): 报表/审计模块接入 Kafka 时启用
fn event_type_name(event: &BusinessEvent) -> &'static str {
    match event {
        BusinessEvent::PurchaseReceiptCompleted { .. } => "PurchaseReceiptCompleted",
        BusinessEvent::SalesOrderShipped { .. } => "SalesOrderShipped",
        BusinessEvent::PaymentCompleted { .. } => "PaymentCompleted",
        BusinessEvent::InventoryAdjusted { .. } => "InventoryAdjusted",
        BusinessEvent::CollectionCompleted { .. } => "CollectionCompleted",
        BusinessEvent::PurchaseOrderApproved { .. } => "PurchaseOrderApproved",
        BusinessEvent::InventoryCountCompleted { .. } => "InventoryCountCompleted",
        BusinessEvent::BpmProcessFinished { .. } => "BpmProcessFinished",
        BusinessEvent::LowStockAlert { .. } => "LowStockAlert",
        BusinessEvent::FinancialIndicatorUpdate { .. } => "FinancialIndicatorUpdate",
        BusinessEvent::MaterialShortageAlert { .. } => "MaterialShortageAlert",
        BusinessEvent::InventoryTransactionCreated { .. } => "InventoryTransactionCreated",
    }
}

/// 为 `BusinessEvent` 增加 `Serialize` / `Deserialize` 派生（仅在 kafka 模块内使用）
///
/// 原 `BusinessEvent` 派生来自 `event_bus.rs`，没有 `Serialize`。这里通过新类型
/// `EventPayload` 包装，再借助 `serde_json` 透明转换，避免在 8 个必需文件之外
/// 改动 `event_bus.rs` 的公共定义。
mod payload_serde {
    use super::{BusinessEvent, ShippedItem};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    /// 与 `BusinessEvent` 字段一一对应的可序列化结构
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    pub enum EventPayload {
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
            amount: Decimal,
        },
        InventoryAdjusted {
            product_id: i32,
            warehouse_id: i32,
            quantity_change: Decimal,
        },
        CollectionCompleted {
            collection_id: i32,
            invoice_id: Option<i32>,
            amount: Decimal,
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
            current_quantity: Decimal,
            reorder_point: Decimal,
            reorder_quantity: Decimal,
        },
        FinancialIndicatorUpdate {
            period: String,
            trigger_source: String,
        },
        MaterialShortageAlert {
            material_id: i32,
            material_name: String,
            material_code: String,
            required_quantity: Decimal,
            available_quantity: Decimal,
            shortage_quantity: Decimal,
            shortage_level: String,
            affected_orders_count: i32,
        },
        InventoryTransactionCreated {
            transaction_id: i32,
            transaction_type: String,
            product_id: i32,
            warehouse_id: i32,
            quantity_meters: Decimal,
            quantity_kg: Decimal,
            source_bill_type: Option<String>,
            source_bill_no: Option<String>,
            source_bill_id: Option<i32>,
            batch_no: String,
            color_no: String,
            created_by: Option<i32>,
        },
    }

    impl From<&BusinessEvent> for EventPayload {
        fn from(event: &BusinessEvent) -> Self {
            match event {
                BusinessEvent::PurchaseReceiptCompleted {
                    receipt_id,
                    order_id,
                    supplier_id,
                } => Self::PurchaseReceiptCompleted {
                    receipt_id: *receipt_id,
                    order_id: *order_id,
                    supplier_id: *supplier_id,
                },
                BusinessEvent::SalesOrderShipped {
                    order_id,
                    customer_id,
                    items,
                } => Self::SalesOrderShipped {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    items: items.clone(),
                },
                BusinessEvent::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                } => Self::PaymentCompleted {
                    payment_id: *payment_id,
                    invoice_id: *invoice_id,
                    amount: *amount,
                },
                BusinessEvent::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                } => Self::InventoryAdjusted {
                    product_id: *product_id,
                    warehouse_id: *warehouse_id,
                    quantity_change: *quantity_change,
                },
                BusinessEvent::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                } => Self::CollectionCompleted {
                    collection_id: *collection_id,
                    invoice_id: *invoice_id,
                    amount: *amount,
                },
                BusinessEvent::PurchaseOrderApproved {
                    order_id,
                    supplier_id,
                } => Self::PurchaseOrderApproved {
                    order_id: *order_id,
                    supplier_id: *supplier_id,
                },
                BusinessEvent::InventoryCountCompleted {
                    count_id,
                    variance_count,
                } => Self::InventoryCountCompleted {
                    count_id: *count_id,
                    variance_count: *variance_count,
                },
                BusinessEvent::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                } => Self::BpmProcessFinished {
                    business_type: business_type.clone(),
                    business_id: *business_id,
                    approved: *approved,
                },
                BusinessEvent::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => Self::LowStockAlert {
                    product_id: *product_id,
                    warehouse_id: *warehouse_id,
                    current_quantity: *current_quantity,
                    reorder_point: *reorder_point,
                    reorder_quantity: *reorder_quantity,
                },
                BusinessEvent::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                } => Self::FinancialIndicatorUpdate {
                    period: period.clone(),
                    trigger_source: trigger_source.clone(),
                },
                BusinessEvent::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                } => Self::MaterialShortageAlert {
                    material_id: *material_id,
                    material_name: material_name.clone(),
                    material_code: material_code.clone(),
                    required_quantity: *required_quantity,
                    available_quantity: *available_quantity,
                    shortage_quantity: *shortage_quantity,
                    shortage_level: shortage_level.clone(),
                    affected_orders_count: *affected_orders_count,
                },
                BusinessEvent::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                } => Self::InventoryTransactionCreated {
                    transaction_id: *transaction_id,
                    transaction_type: transaction_type.clone(),
                    product_id: *product_id,
                    warehouse_id: *warehouse_id,
                    quantity_meters: *quantity_meters,
                    quantity_kg: *quantity_kg,
                    source_bill_type: source_bill_type.clone(),
                    source_bill_no: source_bill_no.clone(),
                    source_bill_id: *source_bill_id,
                    batch_no: batch_no.clone(),
                    color_no: color_no.clone(),
                    created_by: *created_by,
                },
            }
        }
    }

    impl TryFrom<EventPayload> for BusinessEvent {
        type Error = String;
        fn try_from(p: EventPayload) -> Result<Self, Self::Error> {
            Ok(match p {
                EventPayload::PurchaseReceiptCompleted {
                    receipt_id,
                    order_id,
                    supplier_id,
                } => Self::PurchaseReceiptCompleted {
                    receipt_id,
                    order_id,
                    supplier_id,
                },
                EventPayload::SalesOrderShipped {
                    order_id,
                    customer_id,
                    items,
                } => Self::SalesOrderShipped {
                    order_id,
                    customer_id,
                    items,
                },
                EventPayload::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                } => Self::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                },
                EventPayload::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                } => Self::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                },
                EventPayload::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                } => Self::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                },
                EventPayload::PurchaseOrderApproved {
                    order_id,
                    supplier_id,
                } => Self::PurchaseOrderApproved {
                    order_id,
                    supplier_id,
                },
                EventPayload::InventoryCountCompleted {
                    count_id,
                    variance_count,
                } => Self::InventoryCountCompleted {
                    count_id,
                    variance_count,
                },
                EventPayload::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                } => Self::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                },
                EventPayload::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => Self::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                },
                EventPayload::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                } => Self::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                },
                EventPayload::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                } => Self::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                },
                EventPayload::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                } => Self::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                },
            })
        }
    }
}

/// Kafka 后端实现
///
/// 通过 `Arc<KafkaBackendInner>` 共享内部状态；`publish` / `subscribe`
/// 都是 clone 后调用，开销可控。
#[derive(Clone)]
pub struct KafkaBackend {
    inner: Arc<KafkaBackendInner>,
}

struct KafkaBackendInner {
    client: Arc<Client>,
    config: KafkaSettings,
    /// 轮询用：下一次投递使用的 partition
    next_partition: AtomicU64,
}

impl KafkaBackend {
    /// 尝试创建 Kafka 后端。
    ///
    /// 行为：
    /// 1. 解析 `brokers`（逗号分隔）；
    /// 2. 使用 `connect_timeout_ms` 限制连接总耗时；
    /// 3. 若 `auto_create_topic=true`，启动时调用 `create_topic`（已存在时容忍）；
    /// 4. 任意失败 → 返回 `Err` 让上层降级。
    pub async fn try_new(config: &KafkaSettings) -> Result<Self, KafkaError> {
        let brokers: Vec<String> = config
            .brokers
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if brokers.is_empty() {
            return Err(KafkaError(
                "Kafka brokers 配置为空，无法创建客户端".to_string(),
            ));
        }

        // 5s 超时连接，超时立即报错（避免启动卡死）
        let connect_fut = ClientBuilder::new(brokers).build();
        let client = match tokio::time::timeout(
            Duration::from_millis(config.connect_timeout_ms),
            connect_fut,
        )
        .await
        {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                return Err(KafkaError(format!(
                    "连接 Kafka 失败（brokers={}）: {}",
                    config.brokers, e
                )));
            }
            Err(_) => {
                return Err(KafkaError(format!(
                    "连接 Kafka 超时（{}ms, brokers={}）",
                    config.connect_timeout_ms, config.brokers
                )));
            }
        };

        // 自动创建 topic（已存在时 rskafka 会返回错误，但不影响主流程）
        if config.auto_create_topic {
            if let Err(e) = client
                .controller_client()
                .map_err(|e| KafkaError(format!("获取 controller 客户端失败: {}", e)))?
                .create_topic(
                    config.topic.clone(),
                    config.partitions,
                    config.replication_factor,
                    5_000,
                )
                .await
            {
                tracing::warn!(
                    "Kafka 自动创建 topic {} 失败（可能已存在）: {}",
                    config.topic,
                    e
                );
            }
        }

        Ok(Self {
            inner: Arc::new(KafkaBackendInner {
                client: Arc::new(client),
                config: config.clone(),
                next_partition: AtomicU64::new(0),
            }),
        })
    }

    /// 暴露内部 client，便于测试构造 partition client。
    pub fn config(&self) -> &KafkaSettings {
        &self.inner.config
    }

    /// 选择下一个 partition（轮询）
    fn next_partition(&self, total: i32) -> i32 {
        if total <= 0 {
            return 0;
        }
        let idx = self.inner.next_partition.fetch_add(1, Ordering::Relaxed);
        (idx % total as u64) as i32
    }

    /// 异步发布事件到 Kafka
    pub async fn publish(&self, event: BusinessEvent) -> Result<(), KafkaError> {
        let payload = payload_serde::EventPayload::from(&event);
        let payload_json = serde_json::to_vec(&payload)
            .map_err(|e| KafkaError(format!("事件序列化失败: {}", e)))?;

        let partition = self.next_partition(self.inner.config.partitions);
        let partition_client: PartitionClient = self
            .inner
            .client
            .partition_client(
                self.inner.config.topic.clone(),
                partition,
                UnknownTopicHandling::Retry,
            )
            .await
            .map_err(|e| KafkaError(format!("获取 partition 客户端失败: {}", e)))?;

        let record = Record {
            key: None,
            value: Some(payload_json),
            headers: BTreeMap::new(),
            timestamp: chrono::Utc::now(),
        };

        partition_client
            .produce(vec![record], Compression::NoCompression)
            .await
            .map_err(|e| KafkaError(format!("Kafka produce 失败: {}", e)))?;
        Ok(())
    }

    /// 启动消费后台任务并返回事件流
    ///
    /// 后台任务对所有 partition 进行轮询 fetch：
    /// - 起始 offset = 各 partition 的 earliest；
    /// - 拉取间隔 200ms（避免 CPU 100%）；
    /// - 消费失败 → 重新连接（最多 3 次），仍失败则关闭流。
    pub async fn subscribe(
        &self,
    ) -> Result<Box<dyn Stream<Item = BusinessEvent> + Send + Unpin>, KafkaError> {
        let (tx, rx) = mpsc::channel::<BusinessEvent>(256);
        let config = self.inner.config.clone();
        let client = self.inner.client.clone();

        // 后台消费任务：拉取所有 partition，反序列化后推入 mpsc
        tokio::spawn(async move {
            if let Err(e) = run_consumer_loop(client, config, tx.clone()).await {
                tracing::error!("Kafka 消费循环退出: {}", e);
            }
        });

        let stream = KafkaSubscribeStream { rx };
        Ok(Box::new(stream))
    }
}

/// 包装 `mpsc::Receiver` 为 `Stream`
pub struct KafkaSubscribeStream {
    rx: mpsc::Receiver<BusinessEvent>,
}

impl Stream for KafkaSubscribeStream {
    type Item = BusinessEvent;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // `tokio::sync::mpsc::Receiver` 自身实现 `Stream`，直接代理
        self.rx.poll_recv(cx)
    }
}

/// Kafka 消费循环（独立函数，便于测试）
async fn run_consumer_loop(
    client: Arc<Client>,
    config: KafkaSettings,
    tx: mpsc::Sender<BusinessEvent>,
) -> Result<(), KafkaError> {
    let partitions = config.partitions.max(1);
    let mut last_offsets: Vec<i64> = vec![0; partitions as usize];
    let mut initialised = false;
    let mut consecutive_failures: u32 = 0;
    const MAX_FAILURES: u32 = 3;

    loop {
        for partition in 0..partitions {
            if consecutive_failures >= MAX_FAILURES {
                tracing::error!(
                    "Kafka 消费连续失败 {} 次，退出消费循环",
                    consecutive_failures
                );
                return Err(KafkaError("消费连续失败次数超过上限".to_string()));
            }

            let pc = match client
                .partition_client(config.topic.clone(), partition, UnknownTopicHandling::Retry)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    consecutive_failures = consecutive_failures.saturating_add(1);
                    tracing::error!("Kafka 获取 partition {} 客户端失败: {}", partition, e);
                    continue;
                }
            };

            // 首次启动：从 earliest 开始读
            if !initialised {
                match pc.get_offset(OffsetAt::Earliest).await {
                    Ok(off) => last_offsets[partition as usize] = off,
                    Err(e) => {
                        consecutive_failures = consecutive_failures.saturating_add(1);
                        tracing::error!(
                            "Kafka 拉取 partition {} earliest offset 失败: {}",
                            partition,
                            e
                        );
                        continue;
                    }
                }
            }
            initialised = true;

            let current_off = last_offsets[partition as usize];
            let fetch_result = pc
                .fetch_records(current_off, 1_048_576..1_048_576, 1_000)
                .await;
            match fetch_result {
                Ok((records, _high_watermark)) => {
                    consecutive_failures = 0;
                    if let Some(last) = records.last() {
                        last_offsets[partition as usize] = last.offset + 1;
                    }
                    for record_and_off in records {
                        let bytes = match record_and_off.record.value {
                            Some(b) => b,
                            None => continue,
                        };
                        let payload: payload_serde::EventPayload =
                            match serde_json::from_slice(&bytes) {
                                Ok(p) => p,
                                Err(e) => {
                                    tracing::error!(
                                        "Kafka 事件反序列化为 EventPayload 失败: {}",
                                        e
                                    );
                                    continue;
                                }
                            };
                        match BusinessEvent::try_from(payload) {
                            Ok(event) => {
                                if tx.send(event).await.is_err() {
                                    tracing::warn!("Kafka 消费通道已关闭，停止消费");
                                    return Ok(());
                                }
                            }
                            Err(e) => {
                                tracing::error!("Kafka 事件转换 BusinessEvent 失败: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    consecutive_failures = consecutive_failures.saturating_add(1);
                    tracing::error!("Kafka 拉取 partition {} 失败: {}", partition, e);
                }
            }
        }

        // 简单节流：200ms 拉一次
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn sample_event() -> BusinessEvent {
        BusinessEvent::PaymentCompleted {
            payment_id: 1,
            invoice_id: 2,
            amount: Decimal::from_str("100.50").unwrap(),
        }
    }

    /// 验证 `KafkaEventEnvelope` 序列化 / 反序列化 round-trip 一致
    #[test]
    fn test_envelope_round_trip() {
        let event = sample_event();
        let envelope = KafkaEventEnvelope::from_event(&event);
        let json = serde_json::to_string(&envelope).expect("序列化失败");
        let de: KafkaEventEnvelope = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(de.r#type, "PaymentCompleted");
        let restored = de.into_event().expect("还原事件失败");
        match restored {
            BusinessEvent::PaymentCompleted {
                payment_id,
                invoice_id,
                amount,
            } => {
                assert_eq!(payment_id, 1);
                assert_eq!(invoice_id, 2);
                assert_eq!(amount, Decimal::from_str("100.50").unwrap());
            }
            _ => panic!("事件类型不匹配"),
        }
    }

    /// 验证 `EventPayload` 双向转换覆盖所有 variant
    #[test]
    fn test_payload_all_variants_round_trip() {
        let cases: Vec<BusinessEvent> = vec![
            BusinessEvent::PurchaseReceiptCompleted {
                receipt_id: 1,
                order_id: 2,
                supplier_id: 3,
            },
            BusinessEvent::SalesOrderShipped {
                order_id: 1,
                customer_id: 2,
                items: vec![ShippedItem {
                    product_id: 3,
                    quantity: Decimal::from(5),
                }],
            },
            BusinessEvent::PaymentCompleted {
                payment_id: 1,
                invoice_id: 2,
                amount: Decimal::from(10),
            },
            BusinessEvent::InventoryAdjusted {
                product_id: 1,
                warehouse_id: 2,
                quantity_change: Decimal::from(3),
            },
            BusinessEvent::CollectionCompleted {
                collection_id: 1,
                invoice_id: Some(2),
                amount: Decimal::from(20),
            },
            BusinessEvent::PurchaseOrderApproved {
                order_id: 1,
                supplier_id: 2,
            },
            BusinessEvent::InventoryCountCompleted {
                count_id: 1,
                variance_count: 3,
            },
            BusinessEvent::BpmProcessFinished {
                business_type: "purchase_order".to_string(),
                business_id: 1,
                approved: true,
            },
            BusinessEvent::LowStockAlert {
                product_id: 1,
                warehouse_id: 2,
                current_quantity: Decimal::from(1),
                reorder_point: Decimal::from(5),
                reorder_quantity: Decimal::from(10),
            },
            BusinessEvent::FinancialIndicatorUpdate {
                period: "2026-Q2".to_string(),
                trigger_source: "test".to_string(),
            },
            BusinessEvent::MaterialShortageAlert {
                material_id: 1,
                material_name: "棉布".to_string(),
                material_code: "COT-001".to_string(),
                required_quantity: Decimal::from(100),
                available_quantity: Decimal::from(20),
                shortage_quantity: Decimal::from(80),
                shortage_level: "HIGH".to_string(),
                affected_orders_count: 3,
            },
            BusinessEvent::InventoryTransactionCreated {
                transaction_id: 1,
                transaction_type: "PURCHASE_RECEIPT".to_string(),
                product_id: 2,
                warehouse_id: 3,
                quantity_meters: Decimal::from(50),
                quantity_kg: Decimal::from(10),
                source_bill_type: Some("PO".to_string()),
                source_bill_no: Some("PO-001".to_string()),
                source_bill_id: Some(11),
                batch_no: "B-1".to_string(),
                color_no: "RED".to_string(),
                created_by: Some(7),
            },
        ];
        for event in &cases {
            let payload = payload_serde::EventPayload::from(event);
            let bytes = serde_json::to_vec(&payload).expect("序列化失败");
            let restored_payload: payload_serde::EventPayload =
                serde_json::from_slice(&bytes).expect("反序列化失败");
            let restored = BusinessEvent::try_from(restored_payload).expect("转换失败");
            let event_type = event_type_name(event);
            let restored_type = event_type_name(&restored);
            assert_eq!(event_type, restored_type, "事件类型不匹配: {}", event_type);
        }
    }
}
