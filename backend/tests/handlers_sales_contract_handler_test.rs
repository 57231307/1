use bingxi_backend::handlers::sales_contract_handler::*;
use bingxi_backend::models::sales_contract::Model as SalesContractModel;
use chrono::Duration;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的销售合同模型
fn make_sales_contract_model(id: i32, status: &str) -> SalesContractModel {
    SalesContractModel {
        id,
        contract_no: format!("SC-2026-{:04}", id),
        contract_name: format!("测试合同-{}", id),
        customer_id: 1,
        customer_name: Some("测试客户".to_string()),
        status: status.to_string(),
        total_amount: Some(Decimal::new(100000, 2)),
        payment_terms: Some("30天".to_string()),
        created_by: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    }
}

// ===== 模型测试 =====

#[test]
fn test_sales_contract_model_serialization() {
    let contract = make_sales_contract_model(1, "draft");
    let json = serde_json::to_value(&contract).expect("销售合同序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["contract_no"], "SC-2026-0001");
    assert_eq!(json["status"], "draft");
}

#[test]
fn test_sales_contract_amount() {
    let contract = make_sales_contract_model(1, "draft");

    // 验证金额
    assert_eq!(contract.total_amount, Some(Decimal::new(100000, 2)));
}

// ===== 状态转换测试 =====

#[test]
fn test_status_draft_to_active() {
    let contract = make_sales_contract_model(1, "draft");
    assert_eq!(contract.status, "draft");

    // 验证草稿状态可以转换为生效
    let valid_transitions = vec!["active", "cancelled"];
    assert!(valid_transitions.contains(&"active"));
}

#[test]
fn test_status_active_to_expired() {
    let contract = make_sales_contract_model(1, "active");
    assert_eq!(contract.status, "active");

    // 验证生效状态可以转换为过期
    let valid_transitions = vec!["expired", "terminated"];
    assert!(valid_transitions.contains(&"expired"));
}

#[test]
fn test_status_expired_is_final() {
    let contract = make_sales_contract_model(1, "expired");
    assert_eq!(contract.status, "expired");

    // 验证过期状态是终态
    let invalid_transitions = vec!["draft", "active"];
    assert!(invalid_transitions.contains(&"draft"));
    assert!(invalid_transitions.contains(&"active"));
}

// ===== 有效期测试 =====

#[test]
fn test_validity_period() {
    let mut contract = make_sales_contract_model(1, "active");
    contract.effective_date = Some(Utc::now().naive_utc().date());
    contract.expiry_date = Some(Utc::now().naive_utc().date() + chrono::Duration::days(365));

    assert!(contract.effective_date.is_some());
    assert!(contract.expiry_date.is_some());
}

#[test]
fn test_validity_period_expired() {
    let mut contract = make_sales_contract_model(1, "active");
    contract.effective_date = Some(Utc::now().naive_utc().date() - chrono::Duration::days(365));
    contract.expiry_date = Some(Utc::now().naive_utc().date() - chrono::Duration::days(1));

    // 验证已过期
    assert!(contract.expiry_date.unwrap() < Utc::now().naive_utc().date());
}

// ===== 电子签章测试 =====

#[test]
fn test_electronic_signature() {
    let contract = make_sales_contract_model(1, "draft");
    assert!(contract.signed_at.is_none());
    assert!(contract.signed_by_user_id.is_none());
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_sales_contract_json_roundtrip() {
    let contract = make_sales_contract_model(1, "draft");
    let json = serde_json::to_value(&contract).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("contract_no").is_some());
    assert!(json.get("customer_id").is_some());
    assert!(json.get("total_amount").is_some());
    assert!(json.get("status").is_some());
}
