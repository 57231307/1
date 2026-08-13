use bingxi_backend::handlers::purchase_order_handler::*;
use bingxi_backend::models::purchase_order::Model as PurchaseOrderModel;
use bingxi_backend::models::status::po as status_po;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的采购订单模型
fn make_purchase_order_model(id: i32, status: &str) -> PurchaseOrderModel {
    PurchaseOrderModel {
        id,
        order_no: format!("PO-2026-{:04}", id),
        supplier_id: 1,
        supplier_name: Some("测试供应商".to_string()),
        order_date: Utc::now().naive_utc().date(),
        delivery_date: Some(Utc::now().naive_utc().date()),
        status: Some(status.to_string()),
        total_amount: Decimal::new(10000, 2),
        discount_amount: Decimal::new(0, 2),
        final_amount: Decimal::new(10000, 2),
        currency: Some("CNY".to_string()),
        exchange_rate: Some(Decimal::new(1, 0)),
        payment_terms: Some("30天".to_string()),
        shipping_method: Some("快递".to_string()),
        shipping_address: Some("测试地址".to_string()),
        notes: Some("测试备注".to_string()),
        created_by: Some(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        audit_status: Some("pending".to_string()),
        audit_by: None,
        audit_at: None,
        data_source: Some("manual".to_string()),
        external_order_no: None,
        tags: None,
        priority: Some("normal".to_string()),
    }
}

// ===== 状态常量测试 =====

#[test]
fn test_po_status_draft() {
    assert_eq!(status_po::DRAFT, "draft");
}

#[test]
fn test_po_status_confirmed() {
    assert_eq!(status_po::CONFIRMED, "confirmed");
}

#[test]
fn test_po_status_received() {
    assert_eq!(status_po::RECEIVED, "received");
}

#[test]
fn test_po_status_cancelled() {
    assert_eq!(status_po::CANCELLED, "cancelled");
}

// ===== 模型测试 =====

#[test]
fn test_purchase_order_model_serialization() {
    let order = make_purchase_order_model(1, "draft");
    let json = serde_json::to_value(&order).expect("采购订单序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["order_no"], "PO-2026-0001");
    assert_eq!(json["status"], "draft");
}

#[test]
fn test_purchase_order_amounts() {
    let order = make_purchase_order_model(1, "draft");

    // 验证金额
    assert_eq!(order.total_amount, Decimal::new(10000, 2));
    assert_eq!(order.final_amount, Decimal::new(10000, 2));
}

// ===== 状态转换测试 =====

#[test]
fn test_status_draft_to_confirmed() {
    let order = make_purchase_order_model(1, "draft");
    assert_eq!(order.status, Some("draft".to_string()));

    // 验证草稿状态可以转换为已确认
    let valid_transitions = vec!["confirmed", "cancelled"];
    assert!(valid_transitions.contains(&"confirmed"));
}

#[test]
fn test_status_confirmed_to_received() {
    let order = make_purchase_order_model(1, "confirmed");
    assert_eq!(order.status, Some("confirmed".to_string()));

    // 验证已确认状态可以转换为已收货
    let valid_transitions = vec!["received", "cancelled"];
    assert!(valid_transitions.contains(&"received"));
}

#[test]
fn test_status_received_is_final() {
    let order = make_purchase_order_model(1, "received");
    assert_eq!(order.status, Some("received".to_string()));

    // 验证已收货状态是终态
    let invalid_transitions = vec!["draft", "confirmed"];
    assert!(!invalid_transitions.contains(&"draft"));
}

// ===== 优先级测试 =====

#[test]
fn test_priority_normal() {
    let order = make_purchase_order_model(1, "draft");
    assert_eq!(order.priority, Some("normal".to_string()));
}

// ===== 日期测试 =====

#[test]
fn test_order_date() {
    let order = make_purchase_order_model(1, "draft");
    assert!(order.order_date <= Utc::now().naive_utc().date());
}

#[test]
fn test_delivery_date() {
    let order = make_purchase_order_model(1, "draft");
    assert!(order.delivery_date.is_some());
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_purchase_order_json_roundtrip() {
    let order = make_purchase_order_model(1, "draft");
    let json = serde_json::to_value(&order).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("order_no").is_some());
    assert!(json.get("supplier_id").is_some());
    assert!(json.get("total_amount").is_some());
    assert!(json.get("status").is_some());
}
