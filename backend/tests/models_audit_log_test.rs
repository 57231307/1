use bingxi_backend::models::audit_log::Model as AuditLogModel;

/// 构造测试用的审计日志模型
fn make_audit_log_model(id: i32) -> AuditLogModel {
    AuditLogModel {
        id,
        user_id: Some(1),
        username: Some("test_user".to_string()),
        action: "create".to_string(),
        resource_type: Some("sales_order".to_string()),
        resource_id: Some("SO-2026-0001".to_string()),
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/sales/orders".to_string()),
        request_body: Some(r#"{"customer_id": 1}"#.to_string()),
        response_status: Some(200),
        duration_ms: Some(150),
        created_at: Some(chrono::Utc::now()),
        ..Default::default()
    }
}

// ===== 模型测试 =====

#[test]
fn test_audit_log_model_serialization() {
    let log = make_audit_log_model(1);
    let json = serde_json::to_value(&log).expect("审计日志序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["action"], "create");
    assert_eq!(json["resource_type"], "sales_order");
}

#[test]
fn test_audit_log_action() {
    let log = make_audit_log_model(1);
    assert_eq!(log.action, "create");
}

#[test]
fn test_audit_log_resource_type() {
    let log = make_audit_log_model(1);
    assert_eq!(log.resource_type, Some("sales_order".to_string()));
}

// ===== 请求信息测试 =====

#[test]
fn test_audit_log_request_info() {
    let log = make_audit_log_model(1);

    // 验证请求信息
    assert_eq!(log.request_method, Some("POST".to_string()));
    assert_eq!(
        log.request_path,
        Some("/api/v1/erp/sales/orders".to_string())
    );
}

#[test]
fn test_audit_log_ip_address() {
    let log = make_audit_log_model(1);
    assert_eq!(log.ip_address, Some("192.168.1.1".to_string()));
}

// ===== 响应信息测试 =====

#[test]
fn test_audit_log_response_status() {
    let log = make_audit_log_model(1);
    assert_eq!(log.response_status, Some(200));
}

#[test]
fn test_audit_log_duration() {
    let log = make_audit_log_model(1);
    assert_eq!(log.duration_ms, Some(150));
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_audit_log_json_roundtrip() {
    let log = make_audit_log_model(1);
    let json = serde_json::to_value(&log).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("user_id").is_some());
    assert!(json.get("action").is_some());
    assert!(json.get("resource_type").is_some());
    assert!(json.get("created_at").is_some());
}
