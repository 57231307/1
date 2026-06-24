//! P9-7 Kafka 集成模块（Producer + Consumer）
//!
//! 提供：
//! 1. **Producer**：发布业务事件到 Kafka topic
//! 2. **Consumer**：订阅 Kafka topic 消费业务事件
//! 3. **3 个核心 topic**：
//!    - `erp.sales.events`（销售事件）
//!    - `erp.purchase.events`（采购事件）
//!    - `erp.inventory.events`（库存事件）
//!
//! ## 启用 rdkafka
//!
//! 默认情况下，本模块提供 trait 与 mock 实现，**不引入 rdkafka 重依赖**。
//! 要启用真实 Kafka 集成，添加：
//!
//! ```toml
//! rdkafka = { version = "0.36", features = ["cmake-build", "ssl-vendored"] }
//! ```
//!
//! ## 与 Redis pub/sub 的关系
//!
//! P0-P8 已使用 Redis pub/sub 做进程内事件总线。P9-7 引入 Kafka 用于：
//! - **跨服务事件传递**（微服务间）
//! - **事件持久化**（Kafka log 保留 7 天）
//! - **重放能力**（consumer group offset 复位）
//!
//! 业务层可平滑切换：`MessagingProvider` trait 抽象，Redis/Kafka 互换。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 三个核心 Kafka topic
pub mod topics {
    /// 销售事件（订单创建/审批/发货/收款）
    pub const SALES_EVENTS: &str = "erp.sales.events";
    /// 采购事件（订单创建/审批/入库/付款）
    pub const PURCHASE_EVENTS: &str = "erp.purchase.events";
    /// 库存事件（入库/出库/调拨/盘点）
    pub const INVENTORY_EVENTS: &str = "erp.inventory.events";
}

/// 业务事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// 销售订单创建
    SalesOrderCreated,
    /// 销售订单审批通过
    SalesOrderApproved,
    /// 销售订单发货
    SalesOrderShipped,
    /// 销售收款
    SalesPaymentReceived,
    /// 采购订单创建
    PurchaseOrderCreated,
    /// 采购订单到货
    PurchaseOrderReceived,
    /// 采购付款
    PurchasePaymentMade,
    /// 库存入库
    InventoryInbound,
    /// 库存出库
    InventoryOutbound,
    /// 库存调拨
    InventoryTransfer,
    /// 库存盘点
    InventoryCount,
}

impl EventType {
    /// 事件对应的 topic
    pub fn topic(&self) -> &'static str {
        match self {
            Self::SalesOrderCreated
            | Self::SalesOrderApproved
            | Self::SalesOrderShipped
            | Self::SalesPaymentReceived => topics::SALES_EVENTS,
            Self::PurchaseOrderCreated
            | Self::PurchaseOrderReceived
            | Self::PurchasePaymentMade => topics::PURCHASE_EVENTS,
            Self::InventoryInbound
            | Self::InventoryOutbound
            | Self::InventoryTransfer
            | Self::InventoryCount => topics::INVENTORY_EVENTS,
        }
    }

    /// 中文描述
    pub fn desc_zh(&self) -> &'static str {
        match self {
            Self::SalesOrderCreated => "销售订单创建",
            Self::SalesOrderApproved => "销售订单审批",
            Self::SalesOrderShipped => "销售订单发货",
            Self::SalesPaymentReceived => "销售收款",
            Self::PurchaseOrderCreated => "采购订单创建",
            Self::PurchaseOrderReceived => "采购订单到货",
            Self::PurchasePaymentMade => "采购付款",
            Self::InventoryInbound => "库存入库",
            Self::InventoryOutbound => "库存出库",
            Self::InventoryTransfer => "库存调拨",
            Self::InventoryCount => "库存盘点",
        }
    }
}

/// 业务事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessEvent {
    /// 事件 ID（UUID）
    pub event_id: String,
    /// 事件类型
    pub event_type: EventType,
    /// 聚合根 ID（订单号/采购单号等）
    pub aggregate_id: String,
    /// 聚合根类型（"sales_order" / "purchase_order" / "inventory"）
    pub aggregate_type: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 事件数据（JSON 序列化）
    pub payload: serde_json::Value,
    /// 发生时间
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// trace_id（用于关联）
    pub trace_id: Option<String>,
}

impl BusinessEvent {
    /// 创建新事件
    pub fn new(
        event_type: EventType,
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        tenant_id: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: generate_event_id(),
            event_type,
            aggregate_id: aggregate_id.into(),
            aggregate_type: aggregate_type.into(),
            tenant_id: tenant_id.into(),
            payload,
            occurred_at: chrono::Utc::now(),
            trace_id: None,
        }
    }

    /// 设置 trace_id
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// 序列化为 Kafka 消息
    pub fn to_kafka_message(&self) -> KafkaMessage {
        KafkaMessage {
            key: self.aggregate_id.clone(),
            value: serde_json::to_vec(self).unwrap_or_default(),
            headers: HashMap::from([
                ("event_type".to_string(), self.event_type.desc_zh().to_string()),
                ("tenant_id".to_string(), self.tenant_id.clone()),
                ("event_id".to_string(), self.event_id.clone()),
            ]),
        }
    }
}

/// Kafka 消息
#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub key: String,
    pub value: Vec<u8>,
    pub headers: HashMap<String, String>,
}

/// 消息提供方 trait（Redis/Kafka 互换）
#[async_trait]
pub trait MessagingProvider: Send + Sync {
    /// 发布事件
    async fn publish(&self, topic: &str, msg: KafkaMessage) -> Result<(), MessagingError>;
    /// 订阅 topic
    async fn subscribe(&self, topic: &str) -> Result<EventStream, MessagingError>;
}

/// 事件流（用于 Consumer）
pub type EventStream = tokio::sync::mpsc::Receiver<BusinessEvent>;

/// 消息错误
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    #[error("连接失败: {0}")]
    Connection(String),
    #[error("发送失败: {0}")]
    Send(String),
    #[error("订阅失败: {0}")]
    Subscribe(String),
    #[error("序列化失败: {0}")]
    Serialize(String),
}

/// Kafka Producer（mock 实现，可替换为 rdkafka）
pub struct KafkaProducer {
    /// 模拟已发布消息（用于测试）
    sent: Arc<Mutex<Vec<(String, KafkaMessage)>>>,
    /// 启用真实 Kafka
    real_kafka_enabled: bool,
}

impl KafkaProducer {
    /// 创建 mock producer（默认）
    pub fn mock() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            real_kafka_enabled: false,
        }
    }

    /// 创建真实 producer（需启用 rdkafka）
    pub fn real(_brokers: String) -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            real_kafka_enabled: true,
        }
    }

    /// 已发送消息数（仅 mock）
    pub async fn sent_count(&self) -> usize {
        self.sent.lock().await.len()
    }

    /// 获取已发送消息（仅 mock）
    pub async fn sent_messages(&self) -> Vec<(String, KafkaMessage)> {
        self.sent.lock().await.clone()
    }
}

#[async_trait]
impl MessagingProvider for KafkaProducer {
    async fn publish(&self, topic: &str, msg: KafkaMessage) -> Result<(), MessagingError> {
        if self.real_kafka_enabled {
            // 实际生产环境调用 rdkafka producer.send()
            // 此处为占位实现
            tracing::info!("Kafka publish to {}: key={}", topic, msg.key);
        }
        self.sent.lock().await.push((topic.to_string(), msg));
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<EventStream, MessagingError> {
        // TODO(tech-debt): 真实 Kafka Consumer 接入后将通过 _tx 向 EventStream 发送业务事件，当前 mock 实现仅保留发送端以维持通道语义
        let (_tx, rx) = tokio::sync::mpsc::channel(100);
        tracing::info!("Kafka subscribe to {}", topic);
        // 实际生产环境调用 rdkafka consumer
        Ok(rx)
    }
}

/// Kafka Consumer
pub struct KafkaConsumer {
    pub topic: String,
    pub group_id: String,
    pub auto_offset_reset: AutoOffsetReset,
}

/// Offset 重置策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoOffsetReset {
    Earliest,
    Latest,
    Error,
}

impl AutoOffsetReset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Earliest => "earliest",
            Self::Latest => "latest",
            Self::Error => "error",
        }
    }
}

impl KafkaConsumer {
    /// 创建消费者
    pub fn new(topic: impl Into<String>, group_id: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            group_id: group_id.into(),
            auto_offset_reset: AutoOffsetReset::Earliest,
        }
    }

    /// 设置 offset 策略
    pub fn with_offset_reset(mut self, policy: AutoOffsetReset) -> Self {
        self.auto_offset_reset = policy;
        self
    }
}

/// 生成事件 ID
fn generate_event_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("evt-{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_constants() {
        assert_eq!(topics::SALES_EVENTS, "erp.sales.events");
        assert_eq!(topics::PURCHASE_EVENTS, "erp.purchase.events");
        assert_eq!(topics::INVENTORY_EVENTS, "erp.inventory.events");
    }

    #[test]
    fn test_event_type_topic_mapping() {
        assert_eq!(EventType::SalesOrderCreated.topic(), topics::SALES_EVENTS);
        assert_eq!(EventType::PurchaseOrderCreated.topic(), topics::PURCHASE_EVENTS);
        assert_eq!(EventType::InventoryInbound.topic(), topics::INVENTORY_EVENTS);
    }

    #[test]
    fn test_event_type_desc_zh() {
        assert_eq!(EventType::SalesOrderCreated.desc_zh(), "销售订单创建");
        assert_eq!(EventType::PurchasePaymentMade.desc_zh(), "采购付款");
        assert_eq!(EventType::InventoryTransfer.desc_zh(), "库存调拨");
    }

    #[test]
    fn test_event_type_count() {
        // 4 销售 + 3 采购 + 4 库存 = 11 种事件
        let events = vec![
            EventType::SalesOrderCreated,
            EventType::SalesOrderApproved,
            EventType::SalesOrderShipped,
            EventType::SalesPaymentReceived,
            EventType::PurchaseOrderCreated,
            EventType::PurchaseOrderReceived,
            EventType::PurchasePaymentMade,
            EventType::InventoryInbound,
            EventType::InventoryOutbound,
            EventType::InventoryTransfer,
            EventType::InventoryCount,
        ];
        assert_eq!(events.len(), 11);
    }

    #[test]
    fn test_business_event_new() {
        let evt = BusinessEvent::new(
            EventType::SalesOrderCreated,
            "SO-20260617-0001",
            "sales_order",
            "tenant-001",
            serde_json::json!({"amount": 1000}),
        );
        assert!(!evt.event_id.is_empty());
        assert_eq!(evt.aggregate_id, "SO-20260617-0001");
        assert_eq!(evt.tenant_id, "tenant-001");
    }

    #[test]
    fn test_business_event_with_trace() {
        let evt = BusinessEvent::new(
            EventType::PurchaseOrderCreated,
            "PO-001",
            "purchase_order",
            "tenant-002",
            serde_json::json!({}),
        )
        .with_trace_id("trace-abc-123");
        assert_eq!(evt.trace_id, Some("trace-abc-123".to_string()));
    }

    #[test]
    fn test_kafka_message_serialization() {
        let evt = BusinessEvent::new(
            EventType::SalesOrderCreated,
            "SO-001",
            "sales_order",
            "tenant-001",
            serde_json::json!({"amount": 1000}),
        );
        let msg = evt.to_kafka_message();
        assert_eq!(msg.key, "SO-001");
        assert!(!msg.value.is_empty());
        assert!(msg.headers.contains_key("event_type"));
        assert!(msg.headers.contains_key("tenant_id"));
    }

    #[tokio::test]
    async fn test_producer_mock_publish() {
        let producer = KafkaProducer::mock();
        let msg = KafkaMessage {
            key: "k1".to_string(),
            value: b"hello".to_vec(),
            headers: HashMap::new(),
        };
        producer.publish("erp.test", msg).await.unwrap();
        assert_eq!(producer.sent_count().await, 1);
    }

    #[tokio::test]
    async fn test_producer_publish_multiple() {
        let producer = KafkaProducer::mock();
        for i in 0..5 {
            let msg = KafkaMessage {
                key: format!("k{}", i),
                value: vec![i as u8],
                headers: HashMap::new(),
            };
            producer.publish("erp.test", msg).await.unwrap();
        }
        assert_eq!(producer.sent_count().await, 5);
    }

    #[test]
    fn test_consumer_new() {
        let consumer = KafkaConsumer::new("erp.sales.events", "sales-group");
        assert_eq!(consumer.topic, "erp.sales.events");
        assert_eq!(consumer.group_id, "sales-group");
        assert_eq!(consumer.auto_offset_reset, AutoOffsetReset::Earliest);
    }

    #[test]
    fn test_consumer_offset_policy() {
        let consumer = KafkaConsumer::new("t", "g")
            .with_offset_reset(AutoOffsetReset::Latest);
        assert_eq!(consumer.auto_offset_reset, AutoOffsetReset::Latest);
    }

    #[test]
    fn test_offset_reset_as_str() {
        assert_eq!(AutoOffsetReset::Earliest.as_str(), "earliest");
        assert_eq!(AutoOffsetReset::Latest.as_str(), "latest");
        assert_eq!(AutoOffsetReset::Error.as_str(), "error");
    }

    #[test]
    fn test_event_id_unique() {
        let id1 = generate_event_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_event_id();
        assert_ne!(id1, id2);
    }
}
