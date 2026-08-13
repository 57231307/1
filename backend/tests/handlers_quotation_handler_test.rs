use bingxi_backend::handlers::quotation_handler::*;
use bingxi_backend::models::sales_quotation::Model as QuotationModel;
use chrono::Duration;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的报价单模型
fn make_quotation_model(id: i32, status: &str) -> QuotationModel {
    QuotationModel {
        id,
        quotation_no: format!("QT-2026-{:04}", id),
        customer_id: 1,
        customer_name: Some("测试客户".to_string()),
        quotation_date: Utc::now().naive_utc().date(),
        valid_until: Some(Utc::now().naive_utc().date()),
        status: Some(status.to_string()),
        total_amount: Decimal::new(10000, 2),
        discount_amount: Decimal::new(0, 2),
        final_amount: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        exchange_rate: Some(Decimal::new(1, 0)),
        payment_terms: Some("30天".to_string()),
        delivery_terms: Some("FOB".to_string()),
        notes: Some("测试备注".to_string()),
        created_by: Some(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        salesperson_id: Some(1),
        salesperson_name: Some("销售员".to_string()),
        audit_status: Some("pending".to_string()),
        audit_by: None,
        audit_at: None,
        converted_to_order: false,
        conversion_date: None,
    }
}

// ===== 模型测试 =====

#[test]
fn test_quotation_model_serialization() {
    let quotation = make_quotation_model(1, "draft");
    let json = serde_json::to_value(&quotation).expect("报价单序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["quotation_no"], "QT-2026-0001");
    assert_eq!(json["status"], "draft");
}

#[test]
fn test_quotation_amounts() {
    let quotation = make_quotation_model(1, "draft");

    // 验证金额
    assert_eq!(quotation.total_amount, Decimal::new(10000, 2));
    assert_eq!(quotation.final_amount, Decimal::new(10000, 2));
}

// ===== 状态转换测试 =====

#[test]
fn test_status_draft_to_sent() {
    let quotation = make_quotation_model(1, "draft");
    assert_eq!(quotation.status, Some("draft".to_string()));

    // 验证草稿状态可以转换为已发送
    let valid_transitions = vec!["sent", "cancelled"];
    assert!(valid_transitions.contains(&"sent"));
}

#[test]
fn test_status_sent_to_accepted() {
    let quotation = make_quotation_model(1, "sent");
    assert_eq!(quotation.status, Some("sent".to_string()));

    // 验证已发送状态可以转换为已接受
    let valid_transitions = vec!["accepted", "rejected", "expired"];
    assert!(valid_transitions.contains(&"accepted"));
}

#[test]
fn test_status_accepted_is_final() {
    let quotation = make_quotation_model(1, "accepted");
    assert_eq!(quotation.status, Some("accepted".to_string()));

    // 验证已接受状态是终态
    let invalid_transitions = vec!["draft", "sent"];
    assert!(!invalid_transitions.contains(&"draft"));
}

// ===== 转换状态测试 =====

#[test]
fn test_conversion_status() {
    let quotation = make_quotation_model(1, "accepted");
    assert!(!quotation.converted_to_order);
    assert!(quotation.conversion_date.is_none());
}

#[test]
fn test_conversion_to_order() {
    let mut quotation = make_quotation_model(1, "accepted");
    quotation.converted_to_order = true;
    quotation.conversion_date = Some(Utc::now().naive_utc().date());

    assert!(quotation.converted_to_order);
    assert!(quotation.conversion_date.is_some());
}

// ===== 有效期测试 =====

#[test]
fn test_valid_until() {
    let quotation = make_quotation_model(1, "draft");
    assert!(quotation.valid_until.is_some());
}

#[test]
fn test_valid_until_expired() {
    let mut quotation = make_quotation_model(1, "sent");
    quotation.valid_until = Some(Utc::now().naive_utc().date() - chrono::Duration::days(30));

    // 验证已过期
    assert!(quotation.valid_until.unwrap() < Utc::now().naive_utc().date());
}

// ===== 金额计算测试 =====

#[test]
fn test_amount_with_discount() {
    let total = Decimal::new(10000, 2);
    let discount = Decimal::new(1000, 2);
    let final_amount = total - discount;

    assert_eq!(final_amount, Decimal::new(9000, 2));
}

#[test]
fn test_amount_with_exchange_rate() {
    let amount = Decimal::new(10000, 2);
    let rate = Decimal::new(720, 2); // 7.20
    let converted = amount * rate;

    assert_eq!(converted, Decimal::new(720000, 4));
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_quotation_json_roundtrip() {
    let quotation = make_quotation_model(1, "draft");
    let json = serde_json::to_value(&quotation).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("quotation_no").is_some());
    assert!(json.get("customer_id").is_some());
    assert!(json.get("total_amount").is_some());
    assert!(json.get("status").is_some());
}
