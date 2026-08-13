use bingxi_backend::middleware::audit_context::AuditContext;
use sea_orm::ActiveValue;
use sea_orm::entity::prelude::Set;
use serde_json::json;

/// AuditEvent::new 默认值正确
#[test]
fn test_audit_event_new_defaults() {
    let event = AuditEvent::new(OperationType::Login, "auth");
    assert_eq!(event.operation_type, OperationType::Login);
    assert_eq!(event.resource_type, Some("auth".to_string()));
    assert_eq!(event.severity, Severity::Info);
    assert!(event.user_id.is_none());
}

/// 无 ctx 时请求上下文字段全部为空
#[test]
fn test_build_active_model_without_ctx() {
    let event = AuditEvent {
        user_id: Some(42),
        username: Some("alice".to_string()),
        operation_type: OperationType::Update,
        severity: Severity::Warn,
        resource_type: Some("order".to_string()),
        resource_id: Some("1001".to_string()),
        resource_name: Some("订单 A".to_string()),
        description: Some("修改订单金额".to_string()),
        request_method: Some("PUT".to_string()),
        request_path: Some("/api/v1/erp/orders/1001".to_string()),
        before_snapshot: Some(json!({"amount": 100})),
        after_snapshot: Some(json!({"amount": 200})),
    };
    let model = build_active_model(&event, None);
    // 关键字段透传
    if let ActiveValue::Set(s) = model.severity {
        assert_eq!(s, Some("WARN".to_string()));
    } else {
        panic!("severity 应为 Set");
    }
    if let ActiveValue::Set(o) = model.operation_type {
        assert_eq!(o, Some("UPDATE".to_string()));
    } else {
        panic!("operation_type 应为 Set");
    }
    // 无 ctx 时请求上下文为 None
    if let ActiveValue::Set(ip) = model.ip_address {
        assert!(ip.is_none(), "无 ctx 时 ip_address 应为 None");
    }
    if let ActiveValue::Set(rid) = model.request_id {
        assert!(rid.is_none(), "无 ctx 时 request_id 应为 None");
    }
}

/// 有 ctx 时请求上下文自动注入
#[test]
fn test_build_active_model_with_ctx() {
    let event = AuditEvent::new(OperationType::Login, "auth");
    let ctx = AuditContext {
        request_id: "trace-123".to_string(),
        ip_address: "203.0.113.1".to_string(),
        user_agent: "Mozilla/5.0".to_string(),
    };
    let model = build_active_model(&event, Some(&ctx));
    if let ActiveValue::Set(rid) = model.request_id {
        assert_eq!(rid, Some("trace-123".to_string()));
    }
    if let ActiveValue::Set(ip) = model.ip_address {
        assert_eq!(ip, Some("203.0.113.1".to_string()));
    }
    if let ActiveValue::Set(ua) = model.user_agent {
        assert_eq!(ua, Some("Mozilla/5.0".to_string()));
    }
}

/// ctx 字段为空字符串时不会写入数据库（避免污染日志）
#[test]
fn test_build_active_model_with_empty_ctx() {
    let event = AuditEvent::new(OperationType::Logout, "auth");
    let ctx = AuditContext::empty();
    let model = build_active_model(&event, Some(&ctx));
    if let ActiveValue::Set(rid) = model.request_id {
        assert!(rid.is_none(), "空 ctx request_id 应为 None");
    }
    if let ActiveValue::Set(ip) = model.ip_address {
        assert!(ip.is_none(), "空 ctx ip_address 应为 None");
    }
    if let ActiveValue::Set(ua) = model.user_agent {
        assert!(ua.is_none(), "空 ctx user_agent 应为 None");
    }
}

/// 旧字段 old_value/new_value 与新字段 before_snapshot/after_snapshot 内容一致
#[test]
fn test_dual_write_snapshots() {
    let before = json!({"price": 100});
    let after = json!({"price": 200});
    let event = AuditEvent {
        user_id: Some(1),
        username: None,
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("product".to_string()),
        resource_id: Some("1".to_string()),
        resource_name: None,
        description: None,
        request_method: None,
        request_path: None,
        before_snapshot: Some(before.clone()),
        after_snapshot: Some(after.clone()),
    };
    let model = build_active_model(&event, None);
    // old_value/new_value 同步填充
    if let ActiveValue::Set(Some(av)) = model.old_value {
        assert_eq!(av.0, before);
    } else {
        panic!("old_value 应填充 before_snapshot");
    }
    if let ActiveValue::Set(Some(av)) = model.new_value {
        assert_eq!(av.0, after);
    } else {
        panic!("new_value 应填充 after_snapshot");
    }
    // before_snapshot / after_snapshot 也填充
    if let ActiveValue::Set(Some(av)) = model.before_snapshot {
        assert_eq!(av.0, before);
    }
    if let ActiveValue::Set(Some(av)) = model.after_snapshot {
        assert_eq!(av.0, after);
    }
}
