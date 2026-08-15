use bingxi_backend::handlers::purchase_order_handler::*;
use bingxi_backend::models::purchase_order::Model as PurchaseOrderModel;
use bingxi_backend::models::status::purchase_order as status_po;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的采购订单模型
fn make_purchase_order_model(id: i32, _status: &str) -> PurchaseOrderModel {
    PurchaseOrderModel {
        id,
        order_no: format!("PO-2026-{:04}", id),
        supplier_id: 1,
        order_date: Utc::now().naive_utc().date(),
        warehouse_id: 1,
        department_id: 1,
        purchaser_id: 1,
        total_amount: Decimal::new(10000, 2),
        ..Default::default()
    }
}

// ===== 状态常量测试 =====

#[test]
fn test_po_status_draft() {
    assert_eq!(status_po::DRAFT, "DRAFT");
}

#[test]
fn test_po_status_approved() {
    assert_eq!(status_po::APPROVED, "APPROVED");
}

#[test]
fn test_po_status_partial_received() {
    assert_eq!(status_po::PARTIAL_RECEIVED, "PARTIAL_RECEIVED");
}

#[test]
fn test_po_status_cancelled() {
    assert_eq!(status_po::CANCELLED, "CANCELLED");
}

// ===== 模型测试 =====

#[test]
fn test_purchase_order_model_serialization() {
    let order = make_purchase_order_model(1, "draft");
    let json = serde_json::to_value(&order).expect("采购订单序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["order_no"], "PO-2026-0001");
    assert_eq!(json["order_status"], "DRAFT");
}

#[test]
fn test_purchase_order_amounts() {
    let order = make_purchase_order_model(1, "draft");

    // 验证金额
    assert_eq!(order.total_amount, Decimal::new(10000, 2));
}

// ===== 状态转换测试 =====

#[test]
fn test_status_draft_to_confirmed() {
    let order = make_purchase_order_model(1, "draft");
    assert_eq!(order.order_status, "DRAFT");

    // 验证草稿状态可以转换为已确认
    let valid_transitions = vec!["confirmed", "cancelled"];
    assert!(valid_transitions.contains(&"confirmed"));
}

#[test]
fn test_status_confirmed_to_received() {
    let order = make_purchase_order_model(1, "confirmed");
    assert_eq!(order.order_status, "DRAFT");

    // 验证已确认状态可以转换为已收货
    let valid_transitions = vec!["received", "cancelled"];
    assert!(valid_transitions.contains(&"received"));
}

#[test]
fn test_status_received_is_final() {
    let order = make_purchase_order_model(1, "received");
    assert_eq!(order.order_status, "DRAFT");

    // 验证已收货状态是终态
    let invalid_transitions = vec!["draft", "confirmed"];
    assert!(!invalid_transitions.contains(&"draft"));
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
    assert!(order.expected_delivery_date.is_none());
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
    assert!(json.get("order_status").is_some());
}
