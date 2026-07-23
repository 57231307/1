//! Kafka 后端初始化子模块（event_bus_ops/kafka）
//!
//! 从原 `event_bus.rs` 迁移 3 个自由函数：
//! - `init_event_bus_with_kafka_config`：根据 `KafkaSettings` 初始化事件总线后端
//! - `spawn_kafka_consumer`：启动 Kafka 消费桥接后台任务（单次事件 panic 隔离）
//! - `activate_kafka_backend`：将 Kafka 后端写入全局 `EventBusState`

use crate::config::settings::KafkaSettings;
use crate::services::event_bus::{lock_event_bus_state, BusinessEvent};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;

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
        kafka_cfg.brokers, kafka_cfg.topic
    );

    match crate::services::event_kafka::KafkaBackend::try_new(kafka_cfg).await {
        Ok(backend) => {
            let backend = Arc::new(backend);
            let local_tx = {
                let state = lock_event_bus_state();
                state.local_tx.clone()
            };
            let consumer_handle = spawn_kafka_consumer(backend.clone(), local_tx);
            activate_kafka_backend(backend, consumer_handle);
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

/// 启动 Kafka 消费后台任务，把 Kafka 事件桥接到本地 channel
/// 批次 8（2026-06-28）：单次事件处理 panic 隔离
fn spawn_kafka_consumer(
    backend: Arc<crate::services::event_kafka::KafkaBackend>,
    local_tx: broadcast::Sender<BusinessEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match backend.subscribe().await {
            Ok(mut stream) => {
                use futures::stream::StreamExt;
                while let Some(event) = stream.next().await {
                    let result = AssertUnwindSafe(async {
                        if local_tx.send(event).is_err() {
                            tracing::warn!("Kafka 消费桥接：本地 channel 已关闭，停止消费");
                            return false;
                        }
                        true
                    })
                    .catch_unwind()
                    .await;
                    match result {
                        Ok(true) => {} // 继续
                        Ok(false) => break, // channel 关闭，退出
                        Err(panic_payload) => {
                            let panic_msg = panic_payload
                                .downcast_ref::<String>()
                                .map(|s| s.as_str())
                                .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                                .unwrap_or("<非字符串 panic payload>");
                            tracing::error!(
                                panic = %panic_msg,
                                "⚠ Kafka 消费桥接 spawn panic 已被隔离，继续运行（不退出循环）"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Kafka 订阅失败: {}", e);
            }
        }
    })
}

/// 设置 Kafka 后端到全局状态
/// L-27 修复（批次 373 v13 复审）：保存消费桥接 spawn 句柄到 EventBusState，
/// 供 shutdown_event_bus() abort，避免 detached task 泄漏
fn activate_kafka_backend(
    backend: Arc<crate::services::event_kafka::KafkaBackend>,
    consumer_handle: tokio::task::JoinHandle<()>,
) {
    let mut state = lock_event_bus_state();
    state.kafka = Some(backend);
    state.backend_kind.store(1, Ordering::Release);
    state.consumer_handle = Some(consumer_handle);
}
