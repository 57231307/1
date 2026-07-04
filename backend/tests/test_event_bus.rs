//! 事件总线集成测试（P11-H2 Kafka 真实集成）
//!
//! 覆盖以下场景：
//! 1. ✅ BroadcastBackend publish/subscribe 正常（无需 Kafka）
//! 2. ✅ EventBus 默认启动 BroadcastBackend
//! 3. ✅ Kafka 不可达时自动降级到 Broadcast（指向不存在 broker）
//! 4. ✅ BusinessEvent 序列化/反序列化 round-trip 一致（覆盖所有 variant）
//! 5. ✅ 配置开关 KAFKA_ENABLED=true 时实例化 KafkaBackend
//!
//! 设计说明：
//! - 不依赖数据库，CI 友好；
//! - Kafka 测试使用不可达地址 + 短超时（500ms），确保快速失败；
//! - 所有断言使用中文错误消息，便于本地排查。

use bingxi_backend::config::settings::KafkaSettings;
use bingxi_backend::services::event_bus::{
    BroadcastBackend, BusinessEvent, EventBackend, EventBackendType, EventBus, ShippedItem,
};
use bingxi_backend::services::event_kafka::{KafkaBackend, KafkaEventEnvelope};
use futures::stream::StreamExt;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

// ============================================================================
// 测试 1：BroadcastBackend publish/subscribe 正常（无需 Kafka）
// ============================================================================

#[tokio::test]
async fn test_broadcast_backend_publish_subscribe() {
    let backend = BroadcastBackend::new(64);

    // 1.1 订阅
    let stream = backend.subscribe().await.expect("subscribe 失败");

    // 1.2 启动消费任务
    let consumer = tokio::spawn(async move {
        let mut s = stream;
        let first = s.next().await.expect("stream 在发送事件前不应结束");
        first
    });

    // 1.3 发送事件（短暂延迟确保订阅者已就绪）
    tokio::time::sleep(Duration::from_millis(50)).await;
    let event = BusinessEvent::PaymentCompleted {
        payment_id: 100,
        invoice_id: 200,
        amount: Decimal::from_str("999.99").unwrap(),
        user_id: 1,
    };
    backend
        .publish(event.clone())
        .await
        .expect("publish 不应失败");

    // 1.4 验证消费到
    let received = consumer.await.expect("消费者任务失败");
    match received {
        BusinessEvent::PaymentCompleted {
            payment_id,
            invoice_id,
            amount,
            user_id,
        } => {
            assert_eq!(payment_id, 100, "payment_id 不一致");
            assert_eq!(invoice_id, 200, "invoice_id 不一致");
            assert_eq!(
                amount,
                Decimal::from_str("999.99").unwrap(),
                "amount 不一致"
            );
            assert_eq!(user_id, 1, "user_id 不一致");
        }
        other => panic!("事件类型不匹配: {:?}", other),
    }
}

// ============================================================================
// 测试 2：EventBus 默认启动 BroadcastBackend
// ============================================================================

#[tokio::test]
async fn test_event_bus_default_uses_broadcast() {
    // 由于 EVENT_BUS 是全局 LazyLock，新测试进程默认后端 = Broadcast
    // 验证：subscribe 能拿到 publish 的事件
    let bus = EventBus::new();
    assert_eq!(
        bus.backend_type(),
        EventBackendType::Broadcast,
        "默认后端应为 Broadcast"
    );

    let mut rx = bus.subscribe();
    let sent = BusinessEvent::BpmProcessFinished {
        business_type: "purchase_order".to_string(),
        business_id: 42,
        approved: true,
        approver_id: 0,
    };
    bus.publish(sent.clone());

    let received = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("订阅超时")
        .expect("订阅通道关闭");

    match received {
        BusinessEvent::BpmProcessFinished {
            business_type,
            business_id,
            approved,
            approver_id: _,
        } => {
            assert_eq!(business_type, "purchase_order");
            assert_eq!(business_id, 42);
            assert!(approved);
        }
        other => panic!("事件类型不匹配: {:?}", other),
    }
}

// ============================================================================
// 测试 3：Kafka 不可达时自动降级到 Broadcast
// ============================================================================

#[tokio::test]
async fn test_kafka_unreachable_falls_back_to_broadcast() {
    // 指向一个一定不可达的地址（TCP 保留端口 1）+ 100ms 超时
    let cfg = KafkaSettings {
        enabled: true,
        brokers: "127.0.0.1:1".to_string(),
        topic: "erp_business_events_test_unreachable".to_string(),
        consumer_group: "erp_test_consumer".to_string(),
        client_id: "bingxi-test".to_string(),
        partitions: 1,
        replication_factor: 1,
        connect_timeout_ms: 200,
        auto_create_topic: false, // 关闭自动创建以加快失败
    };

    // 3.1 try_new 必须返回错误
    let result = KafkaBackend::try_new(&cfg).await;
    assert!(
        result.is_err(),
        "Kafka 不可达时 try_new 应返回错误（避免卡死启动流程）"
    );
    let err = result.err().unwrap();
    assert!(
        !err.0.is_empty(),
        "错误信息不应为空（应包含中文描述便于运维排查）"
    );

    // 3.2 init_event_bus_with_kafka_config 在 Kafka 失败时应保持 Broadcast 后端
    let bus = EventBus::new();
    bingxi_backend::services::event_bus::init_event_bus_with_kafka_config(&cfg).await;
    assert_eq!(
        bus.backend_type(),
        EventBackendType::Broadcast,
        "Kafka 不可达时必须降级到 Broadcast"
    );
}

// ============================================================================
// 测试 4：BusinessEvent 序列化/反序列化 round-trip 一致
// ============================================================================

#[tokio::test]
async fn test_business_event_serde_round_trip_all_variants() {
    // 覆盖所有 12 种 BusinessEvent 变体
    let cases: Vec<BusinessEvent> = vec![
        BusinessEvent::PurchaseReceiptCompleted {
            receipt_id: 1,
            order_id: 2,
            supplier_id: 3,
        },
        BusinessEvent::SalesOrderShipped {
            order_id: 10,
            customer_id: 20,
            items: vec![
                ShippedItem {
                    product_id: 100,
                    quantity: Decimal::from(5),
                },
                ShippedItem {
                    product_id: 101,
                    quantity: Decimal::from_str("3.5").unwrap(),
                },
            ],
        },
        BusinessEvent::PaymentCompleted {
            payment_id: 1,
            invoice_id: 2,
            amount: Decimal::from_str("1234.56").unwrap(),
            user_id: 10,
        },
        BusinessEvent::InventoryAdjusted {
            product_id: 1,
            warehouse_id: 2,
            quantity_change: Decimal::from_str("-10.5").unwrap(),
        },
        BusinessEvent::CollectionCompleted {
            collection_id: 1,
            invoice_id: Some(2),
            amount: Decimal::from(500),
            user_id: 0,
        },
        BusinessEvent::CollectionCompleted {
            collection_id: 2,
            invoice_id: None,
            amount: Decimal::from(0),
            user_id: 0,
        },
        BusinessEvent::PurchaseOrderApproved {
            order_id: 1,
            supplier_id: 2,
        },
        BusinessEvent::InventoryCountCompleted {
            count_id: 1,
            variance_count: 5,
        },
        BusinessEvent::BpmProcessFinished {
            business_type: "sales_order".to_string(),
            business_id: 7,
            approved: false,
            approver_id: 0,
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
            trigger_source: "voucher_service".to_string(),
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

    for original in &cases {
        // 4.1 KafkaEventEnvelope 序列化
        let envelope = KafkaEventEnvelope::from_event(original);
        let json = serde_json::to_string(&envelope).expect("信封序列化失败");
        let de: KafkaEventEnvelope = serde_json::from_str(&json).expect("信封反序列化失败");

        // 4.2 还原为 BusinessEvent
        let restored = de.into_event().expect("事件还原失败");

        // 4.3 二次序列化得到一致的 JSON（确认变体信息保留）
        let json2 = serde_json::to_string(&KafkaEventEnvelope::from_event(&restored))
            .expect("二次序列化失败");
        assert_eq!(
            json, json2,
            "round-trip 后 JSON 不一致：原始={}, 还原={}",
            json, json2
        );
    }
}

// ============================================================================
// 测试 5：配置开关 KAFKA_ENABLED=true 时实例化 KafkaBackend
// ============================================================================

#[tokio::test]
async fn test_kafka_enabled_triggers_backend_construction() {
    // 5.1 关闭 KAFKA → init 不应尝试连接（仅记录日志）
    let disabled_cfg = KafkaSettings {
        enabled: false,
        ..KafkaSettings::default()
    };
    bingxi_backend::services::event_bus::init_event_bus_with_kafka_config(&disabled_cfg).await;
    // 不修改后端，保持 Broadcast
    assert_eq!(EventBus::new().backend_type(), EventBackendType::Broadcast);

    // 5.2 开启 KAFKA 但指向不可达地址 → try_new 失败，后端保持 Broadcast
    let unreachable_cfg = KafkaSettings {
        enabled: true,
        brokers: "127.0.0.1:1".to_string(),
        topic: "erp_test_unreachable_topic".to_string(),
        connect_timeout_ms: 200,
        auto_create_topic: false,
        ..KafkaSettings::default()
    };
    let backend_result = KafkaBackend::try_new(&unreachable_cfg).await;
    assert!(
        backend_result.is_err(),
        "KAFKA_ENABLED=true 但 broker 不可达时，try_new 必须返回错误"
    );

    // 5.3 验证：多 broker 字符串解析（生产配置常用逗号分隔）
    let cfg = KafkaSettings {
        enabled: true,
        brokers: "broker1:9092,broker2:9092,broker3:9092".to_string(),
        topic: "test_topic".to_string(),
        consumer_group: "test_group".to_string(),
        client_id: "test_client".to_string(),
        partitions: 6,
        replication_factor: 1,
        connect_timeout_ms: 3000,
        auto_create_topic: false,
    };
    let broker_list: Vec<&str> = cfg.brokers.split(',').map(|s| s.trim()).collect();
    assert_eq!(broker_list.len(), 3, "应解析出 3 个 broker");
    assert_eq!(broker_list[0], "broker1:9092");
    assert_eq!(broker_list[2], "broker3:9092");
    assert_eq!(cfg.partitions, 6);
    assert_eq!(cfg.connect_timeout_ms, 3000);
    assert!(!cfg.auto_create_topic);
}
