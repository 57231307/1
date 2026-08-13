use bingxi_backend::middleware::audit_context::AuditContext;
use bingxi_backend::models::audit_log::{OperationType, Severity};
use bingxi_backend::services::audit_log_service::{AuditEvent, build_active_model};
use sea_orm::ActiveValue;
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

/// 有 ctx 时请求上下文字段正确填充
#[test]
fn test_build_active_model_with_ctx() {
    let event = AuditEvent::new(OperationType::Login, "auth");
    let ctx = AuditContext {
        request_id: Some("req-123".to_string()),
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
    };
    let model = build_active_model(&event, Some(&ctx));
    if let ActiveValue::Set(rid) = model.request_id {
        assert_eq!(rid, Some("req-123".to_string()));
    } else {
        panic!("request_id 应为 Set");
    }
    if let ActiveValue::Set(ip) = model.ip_address {
        assert_eq!(ip, Some("192.168.1.1".to_string()));
    } else {
        panic!("ip_address 应为 Set");
    }
}

/// 不同 OperationType 序列化正确
#[test]
fn test_operation_type_serialization() {
    assert_eq!(OperationType::Login.to_string(), "LOGIN");
    assert_eq!(OperationType::Create.to_string(), "CREATE");
    assert_eq!(OperationType::Update.to_string(), "UPDATE");
    assert_eq!(OperationType::Delete.to_string(), "DELETE");
}

/// 不同 Severity 序列化正确
#[test]
fn test_severity_serialization() {
    assert_eq!(Severity::Info.to_string(), "INFO");
    assert_eq!(Severity::Warn.to_string(), "WARN");
    assert_eq!(Severity::Error.to_string(), "ERROR");
    assert_eq!(Severity::Critical.to_string(), "CRITICAL");
}
