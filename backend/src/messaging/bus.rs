//! P9-7 消息总线
//!
//! 业务事件发布与订阅的统一接口

use async_trait::async_trait;
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

/// 业务事件（重新导出 kafka 模块）
pub use super::kafka::{BusinessEvent, EventType, KafkaMessage, MessagingError, MessagingProvider};

/// 事件处理器 trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// 事件类型过滤
    fn event_type(&self) -> EventType;
    /// 处理事件
    async fn handle(&self, event: &BusinessEvent) -> Result<(), MessagingError>;
}

/// 事件总线
pub struct EventBus {
    provider: Arc<dyn MessagingProvider>,
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl EventBus {
    /// 创建事件总线
    pub fn new(provider: Arc<dyn MessagingProvider>) -> Self {
        Self {
            provider,
            handlers: Vec::new(),
        }
    }

    /// 注册事件处理器
    pub fn register(&mut self, handler: Arc<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// 发布事件
    pub async fn publish(&self, event: BusinessEvent) -> Result<(), MessagingError> {
        let topic = event.event_type.topic();
        let msg = event.to_kafka_message();
        self.provider.publish(topic, msg).await
    }

    /// 启动订阅
    pub async fn start_subscribing(&self) -> Result<(), MessagingError> {
        for handler in &self.handlers {
            let topic = handler.event_type().topic();
            let mut stream = self.provider.subscribe(topic).await?;
            let handler = handler.clone();
            tokio::spawn(async move {
                while let Some(event) = stream.recv().await {
                    // 批次 8（2026-06-28）：单次事件处理 panic 隔离
                    let result = AssertUnwindSafe(async {
                        if event.event_type == handler.event_type() {
                            if let Err(e) = handler.handle(&event).await {
                                tracing::error!("Event handler failed: {}", e);
                            }
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
                            "⚠ 事件订阅 spawn panic 已被隔离，继续运行（不退出循环）"
                        );
                    }
                }
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::kafka::KafkaProducer;

    #[tokio::test]
    async fn test_event_bus_publish() {
        let producer = Arc::new(KafkaProducer::mock());
        let bus = EventBus::new(producer.clone());
        let event = BusinessEvent::new(
            EventType::SalesOrderCreated,
            "SO-001",
            "sales_order",
            "tenant-001",
            serde_json::json!({}),
        );
        bus.publish(event).await.unwrap();
        assert_eq!(producer.sent_count().await, 1);
    }

    #[test]
    fn test_event_bus_register() {
        let producer = Arc::new(KafkaProducer::mock());
        let mut bus = EventBus::new(producer);
        // 注册处理器（空 handler 用于测试）
        assert_eq!(bus.handlers.len(), 0);
    }
}
