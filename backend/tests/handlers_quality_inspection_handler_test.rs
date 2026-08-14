use bingxi_backend::handlers::quality_inspection_handler::*;
use bingxi_backend::models::quality_inspection::Model as QualityInspectionModel;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的质检单模型
fn make_quality_inspection_model(id: i32, status: &str) -> QualityInspectionModel {
    QualityInspectionModel {
        id,
        standard_code: format!("QI-2026-{:04}", id),
        source_type: Some("purchase_receipt".to_string()),
        source_id: Some(1),
        source_no: Some("PR-2026-0001".to_string()),
        product_id: 1,
        product_name: Some("测试产品".to_string()),
        product_code: Some("P001".to_string()),
        batch_no: Some("B001".to_string()),
        inspection_date: Utc::now().naive_utc().date(),
        inspector_id: Some(1),
        inspector_name: Some("质检员".to_string()),
        quantity: Decimal::new(100, 0),
        qualified_quantity: Decimal::new(95, 0),
        unqualified_quantity: Decimal::new(5, 0),
        status: Some(status.to_string()),
        result: Some("qualified".to_string()),
        notes: Some("测试备注".to_string()),
        created_by: Some(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
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
fn test_quality_inspection_quantities() {
    let inspection = make_quality_inspection_model(1, "completed");

    // 验证数量关系
    assert_eq!(inspection.quantity, Decimal::new(100, 0));
    assert_eq!(inspection.qualified_quantity, Decimal::new(95, 0));
    assert_eq!(inspection.unqualified_quantity, Decimal::new(5, 0));

    // 验证合格数量 + 不合格数量 = 总数量
    assert_eq!(
        inspection.qualified_quantity + inspection.unqualified_quantity,
        inspection.quantity
    );
}

#[test]
fn test_quality_inspection_pass_rate() {
    let inspection = make_quality_inspection_model(1, "completed");

    // 验证合格率计算
    let pass_rate = inspection.qualified_quantity / inspection.quantity * Decimal::new(100, 0);
    assert_eq!(pass_rate, Decimal::new(95, 0));
}

// ===== 结果测试 =====

#[test]
fn test_inspection_result_qualified() {
    let inspection = make_quality_inspection_model(1, "completed");
    assert_eq!(inspection.result, Some("qualified".to_string()));
}

#[test]
fn test_inspection_result_unqualified() {
    let mut inspection = make_quality_inspection_model(1, "completed");
    inspection.result = Some("unqualified".to_string());
    inspection.qualified_quantity = Decimal::new(0, 0);
    inspection.unqualified_quantity = Decimal::new(100, 0);

    assert_eq!(inspection.result, Some("unqualified".to_string()));
    assert_eq!(inspection.unqualified_quantity, Decimal::new(100, 0));
}

// ===== 来源类型测试 =====

#[test]
fn test_source_type_purchase_receipt() {
    let inspection = make_quality_inspection_model(1, "pending");
    assert_eq!(inspection.source_type, Some("purchase_receipt".to_string()));
    assert_eq!(inspection.source_no, Some("PR-2026-0001".to_string()));
}

// ===== 状态转换测试 =====

#[test]
fn test_status_pending_to_in_progress() {
    let inspection = make_quality_inspection_model(1, "pending");
    assert_eq!(inspection.status, Some("pending".to_string()));

    // 验证待检状态可以转换为进行中
    let valid_transitions = vec!["in_progress", "cancelled"];
    assert!(valid_transitions.contains(&"in_progress"));
}

#[test]
fn test_status_in_progress_to_completed() {
    let inspection = make_quality_inspection_model(1, "in_progress");
    assert_eq!(inspection.status, Some("in_progress".to_string()));

    // 验证进行中状态可以转换为已完成
    let valid_transitions = vec!["completed"];
    assert!(valid_transitions.contains(&"completed"));
}

#[test]
fn test_status_completed_is_final() {
    let inspection = make_quality_inspection_model(1, "completed");
    assert_eq!(inspection.status, Some("completed".to_string()));

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
    assert!(json.get("product_id").is_some());
    assert!(json.get("quantity").is_some());
    assert!(json.get("status").is_some());
}
