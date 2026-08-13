use bingxi_backend::models::financial_analysis_result::*;
use bingxi_backend::models::slow_query::*;
use chrono::Utc;


/// 模型字段默认值验证
#[test]
fn test_model_default_values() {
    let m = Model::default();
    assert_eq!(m.id, 0);
    assert_eq!(m.query_text, String::new());
    assert_eq!(m.execution_time_ms, 0.0);
    assert_eq!(m.calls, 0);
    assert_eq!(m.rows_examined, 0);
    assert!(m.database_name.is_none());
}

/// Model → DTO 转换正确性
#[test]
fn test_model_to_dto_conversion() {
    let captured = chrono::Utc::now();
    let m = Model {
        id: 100,
        query_text: "SELECT * FROM users".to_string(),
        execution_time_ms: 250.5,
        calls: 42,
        rows_examined: 1024,
        database_name: Some("bingxi_erp".to_string()),
        captured_at: captured,
        optimization_status: None,
        assigned_to: None,
        jira_ticket: None,
    };
    let dto: SlowQueryDto = m.into();
    assert_eq!(dto.id, 100);
    assert_eq!(dto.query_text, "SELECT * FROM users");
    assert_eq!(dto.execution_time_ms, 250.5);
    assert_eq!(dto.calls, 42);
    assert_eq!(dto.rows_examined, 1024);
    assert_eq!(dto.database_name, Some("bingxi_erp".to_string()));
    // captured_at 已转成 RFC3339 字符串（包含时区偏移）
    assert!(dto.captured_at.contains("T"));
    assert!(dto.captured_at.contains("+") || dto.captured_at.ends_with("Z"));
}

/// DTO 序列化/反序列化（验证 JSON 字段命名）
#[test]
fn test_dto_serialize() {
    let dto = SlowQueryDto {
        id: 1,
        query_text: "SELECT 1".to_string(),
        execution_time_ms: 100.0,
        calls: 1,
        rows_examined: 1,
        database_name: None,
        captured_at: "2026-06-18T10:00:00+00:00".to_string(),
        optimization_status: None,
        assigned_to: None,
        jira_ticket: None,
    };
    let json = serde_json::to_string(&dto).expect("序列化应成功");
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"query_text\":\"SELECT 1\""));
    assert!(json.contains("\"execution_time_ms\":100.0"));
    assert!(json.contains("\"calls\":1"));
    // 验证可反序列化
    let round: SlowQueryDto = serde_json::from_str(&json).expect("反序列化应成功");
    assert_eq!(round.id, 1);
    assert_eq!(round.query_text, "SELECT 1");
}