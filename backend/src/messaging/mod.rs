//! P9-7 消息模块
//!
//! Kafka / Redis 统一抽象，提供业务事件发布与订阅能力

pub mod bus;
pub mod kafka;

pub use bus::{BusinessEvent, EventBus, EventHandler, EventType, KafkaMessage, MessagingError, MessagingProvider};
