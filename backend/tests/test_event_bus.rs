//! 事件总线贯通集成测试（V15 P2 B04-P2-6）
//!
//! 覆盖 BusinessModeChanged / OrderBusinessModeLinked 的发布-订阅闭环。
//! 事件总线默认后端为进程内 tokio::broadcast（CI 友好，无需 Kafka/Redis），
//! publish/subscribe 直接走本地 channel，可纯异步测试验证事件贯通。
//! start_event_listener（需 DB + SearchClient）的下游业务分发由 CI 集成环境执行。

use bingxi_backend::services::event_bus::{BusinessEvent, EVENT_BUS};
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

// 测试夹具（规则 6：mock 数据抽取到 fixtures，禁止硬编码）
mod fixtures {
    pub const MODE_ID: i32 = 401;
    pub const CHANGED_BY: i32 = 9001;
    pub const DOCUMENT_ID: i32 = 7701;

    pub fn mode_code() -> String {
        "MIXED_PROCESS".to_string()
    }
    pub fn mode_name() -> String {
        "混合加工模式".to_string()
    }
    pub fn document_type() -> String {
        "sales_order".to_string()
    }
    pub fn document_no() -> String {
        "SO-2026-0731-0001".to_string()
    }
}

/// 串行化事件总线测试：EVENT_BUS 为全局单例，并发测试会互相投递/消费事件，
/// 通过该锁保证每个测试的 订阅→发布→接收 临界区互斥。
static EVENT_BUS_TEST_LOCK: Mutex<()> = Mutex::new(());

/// 接收事件（500ms 超时），跳过 Lagged（广播缓冲溢出时容错重试）。
async fn recv_event(receiver: &mut broadcast::Receiver<BusinessEvent>) -> BusinessEvent {
    loop {
        match tokio::time::timeout(Duration::from_millis(500), receiver.recv()).await {
            Ok(Ok(event)) => return event,
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => {
                panic!("事件 channel 已关闭，无法接收")
            }
            Err(_) => panic!("500ms 内未收到预期事件"),
        }
    }
}

/// 验证 BusinessModeChanged 事件发布-订阅闭环：订阅→发布→接收字段一致。
#[tokio::test]
async fn test_business_mode_changed_publish_subscribe() {
    let _guard = EVENT_BUS_TEST_LOCK.lock().await;
    // 订阅必须在发布之前：broadcast 仅投递订阅之后的事件
    let mut receiver = EVENT_BUS.subscribe();

    EVENT_BUS.publish(BusinessEvent::BusinessModeChanged {
        mode_id: fixtures::MODE_ID,
        mode_code: fixtures::mode_code(),
        mode_name: fixtures::mode_name(),
        changed_by: fixtures::CHANGED_BY,
    });

    match recv_event(&mut receiver).await {
        BusinessEvent::BusinessModeChanged {
            mode_id,
            mode_code,
            mode_name,
            changed_by,
        } => {
            assert_eq!(mode_id, fixtures::MODE_ID);
            assert_eq!(mode_code, fixtures::mode_code());
            assert_eq!(mode_name, fixtures::mode_name());
            assert_eq!(changed_by, fixtures::CHANGED_BY);
        }
        other => panic!("期望 BusinessModeChanged，实际收到 {:?}", other),
    }
}

/// 验证 OrderBusinessModeLinked 事件发布-订阅闭环：订阅→发布→接收字段一致。
#[tokio::test]
async fn test_order_business_mode_linked_publish_subscribe() {
    let _guard = EVENT_BUS_TEST_LOCK.lock().await;
    let mut receiver = EVENT_BUS.subscribe();

    EVENT_BUS.publish(BusinessEvent::OrderBusinessModeLinked {
        document_type: fixtures::document_type(),
        document_id: fixtures::DOCUMENT_ID,
        document_no: fixtures::document_no(),
        mode_id: fixtures::MODE_ID,
        mode_code: fixtures::mode_code(),
        mode_name: fixtures::mode_name(),
    });

    match recv_event(&mut receiver).await {
        BusinessEvent::OrderBusinessModeLinked {
            document_type,
            document_id,
            document_no,
            mode_id,
            mode_code,
            mode_name,
        } => {
            assert_eq!(document_type, fixtures::document_type());
            assert_eq!(document_id, fixtures::DOCUMENT_ID);
            assert_eq!(document_no, fixtures::document_no());
            assert_eq!(mode_id, fixtures::MODE_ID);
            assert_eq!(mode_code, fixtures::mode_code());
            assert_eq!(mode_name, fixtures::mode_name());
        }
        other => panic!("期望 OrderBusinessModeLinked，实际收到 {:?}", other),
    }
}

/// 验证广播语义：发布之后才订阅的接收者不应收到该事件。
#[tokio::test]
async fn test_late_subscriber_does_not_receive_event() {
    let _guard = EVENT_BUS_TEST_LOCK.lock().await;
    // 先发布，再订阅（无活跃订阅者时 publish 仅记 warn 日志，事件被丢弃）
    EVENT_BUS.publish(BusinessEvent::BusinessModeChanged {
        mode_id: fixtures::MODE_ID,
        mode_code: fixtures::mode_code(),
        mode_name: fixtures::mode_name(),
        changed_by: fixtures::CHANGED_BY,
    });
    let mut late_receiver = EVENT_BUS.subscribe();

    let result = tokio::time::timeout(Duration::from_millis(200), late_receiver.recv()).await;
    assert!(
        result.is_err(),
        "订阅晚于发布的接收者不应收到事件（broadcast 仅投递订阅之后的事件）"
    );
}
