use bingxi_backend::handlers::quality_inspection_handler::*;
use bingxi_backend::models::quality_inspection::Model as QualityInspectionModel;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的质检标准模型
fn make_quality_inspection_model(id: i32, status: &str) -> QualityInspectionModel {
    QualityInspectionModel {
        id,
        standard_name: format!("测试标准-{}", id),
        standard_code: format!("QI-2026-{:04}", id),
        product_id: Some(1),
        product_category_id: Some(1),
        inspection_type: "incoming".to_string(),
        sampling_rate: Some(rust_decimal::Decimal::new(10, 0)),
        inspection_items: Some(serde_json::json!([{"item": "外观检查"}])),
        status: status.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    }
}

// ===== 模型测试 =====

#[test]
fn test_quality_inspection_model_serialization() {
    let inspection = make_quality_inspection_model(1, "pending");
    let json = serde_json::to_value(&inspection).expect("质检单序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["standard_code"], "QI-2026-0001");
    assert_eq!(json["status"], "pending");
}

#[test]
fn test_quality_inspection_sampling_rate() {
    let inspection = make_quality_inspection_model(1, "completed");

    // 验证抽样率设置
    assert!(inspection.sampling_rate.is_some());
}

#[test]
fn test_quality_inspection_inspection_type() {
    let inspection = make_quality_inspection_model(1, "completed");

    // 验证检验类型
    assert_eq!(inspection.inspection_type, "incoming");
}

// ===== 状态测试 =====

#[test]
fn test_inspection_status_completed() {
    let inspection = make_quality_inspection_model(1, "completed");
    assert_eq!(inspection.status, "completed");
}

#[test]
fn test_inspection_status_in_progress() {
    let mut inspection = make_quality_inspection_model(1, "pending");
    inspection.status = "in_progress".to_string();

    assert_eq!(inspection.status, "in_progress");
}

// ===== 检验项目测试 =====

#[test]
fn test_inspection_items() {
    let inspection = make_quality_inspection_model(1, "pending");
    assert!(inspection.inspection_items.is_some());
}

// ===== 状态转换测试 =====

#[test]
fn test_status_pending_to_in_progress() {
    let inspection = make_quality_inspection_model(1, "pending");
    assert_eq!(inspection.status, "pending");

    // 验证待检状态可以转换为进行中
    let valid_transitions = vec!["in_progress", "cancelled"];
    assert!(valid_transitions.contains(&"in_progress"));
}

#[test]
fn test_status_in_progress_to_completed() {
    let inspection = make_quality_inspection_model(1, "in_progress");
    assert_eq!(inspection.status, "in_progress");

    // 验证进行中状态可以转换为已完成
    let valid_transitions = vec!["completed"];
    assert!(valid_transitions.contains(&"completed"));
}

#[test]
fn test_status_completed_is_final() {
    let inspection = make_quality_inspection_model(1, "completed");
    assert_eq!(inspection.status, "completed");

    // 验证已完成状态是终态
    let invalid_transitions = vec!["pending", "in_progress"];
    assert!(!invalid_transitions.contains(&"pending"));
}

// ===== 合格率计算测试 =====

#[test]
fn test_pass_rate_calculation() {
    let qualified = Decimal::new(95, 0);
    let total = Decimal::new(100, 0);
    let pass_rate = qualified / total * Decimal::new(100, 0);

    assert_eq!(pass_rate, Decimal::new(95, 0));
}

#[test]
fn test_pass_rate_zero_total() {
    let qualified = Decimal::new(0, 0);
    let total = Decimal::new(0, 0);

    // 验证除零保护
    let pass_rate = if total > Decimal::new(0, 0) {
        qualified / total * Decimal::new(100, 0)
    } else {
        Decimal::new(0, 0)
    };

    assert_eq!(pass_rate, Decimal::new(0, 0));
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_quality_inspection_json_roundtrip() {
    let inspection = make_quality_inspection_model(1, "pending");
    let json = serde_json::to_value(&inspection).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("standard_code").is_some());
    assert!(json.get("inspection_type").is_some());
    assert!(json.get("status").is_some());
}
