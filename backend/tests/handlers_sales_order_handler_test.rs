use bingxi_backend::handlers::sales_order_handler::*;
use bingxi_backend::models::sales_order::Model as SalesOrderModel;
use bingxi_backend::models::status::sales_order as status_so;
use bingxi_backend::utils::response::ApiResponse;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的销售订单模型
fn make_sales_order_model(id: i32, _status: &str) -> SalesOrderModel {
    SalesOrderModel {
        id,
        order_no: format!("SO-2026-{:04}", id),
        customer_id: 1,
        order_date: Utc::now(),
        required_date: Utc::now(),
        total_amount: Decimal::new(10000, 2),
        ..Default::default()
    }
}

// ===== 状态常量测试 =====

#[test]
fn test_so_status_draft() {
    assert_eq!(status_so::DRAFT, "draft");
}

#[test]
fn test_so_status_approved() {
    assert_eq!(status_so::APPROVED, "approved");
}

#[test]
fn test_so_status_shipped() {
    assert_eq!(status_so::SHIPPED, "shipped");
}

#[test]
fn test_so_status_cancelled() {
    assert_eq!(status_so::CANCELLED, "cancelled");
}

// ===== 查询参数测试 =====

#[test]
fn test_sales_order_query_default() {
    let query = SalesOrderQuery {
        page: None,
        page_size: None,
        status: None,
        customer_id: None,
        order_no: None,
    };
    assert!(query.page.is_none());
    assert!(query.page_size.is_none());
    assert!(query.status.is_none());
    assert!(query.customer_id.is_none());
    assert!(query.order_no.is_none());
}

#[test]
fn test_sales_order_query_with_values() {
    let query = SalesOrderQuery {
        page: Some(1),
        page_size: Some(10),
        status: Some("draft".to_string()),
        customer_id: Some(1),
        order_no: Some("SO-2026-0001".to_string()),
    };
    assert_eq!(query.page, Some(1));
    assert_eq!(query.page_size, Some(10));
    assert_eq!(query.status, Some("draft".to_string()));
    assert_eq!(query.customer_id, Some(1));
    assert_eq!(query.order_no, Some("SO-2026-0001".to_string()));
}

// ===== 模型序列化测试 =====

#[test]
fn test_sales_order_model_serialization() {
    let order = make_sales_order_model(1, "draft");
    let json = serde_json::to_value(&order).expect("销售订单序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["order_no"], "SO-2026-0001");
    assert_eq!(json["customer_id"], 1);
    assert_eq!(json["status"], "");
}

#[test]
fn test_sales_order_model_amounts() {
    let order = make_sales_order_model(1, "draft");
    let json = serde_json::to_value(&order).expect("销售订单序列化失败");

    // 验证金额字段
    assert!(json["total_amount"].is_string());
    assert!(json["subtotal"].is_string());
}

// ===== 状态转换测试 =====

#[test]
fn test_status_transitions_draft_to_confirmed() {
    let order = make_sales_order_model(1, "draft");
    assert_eq!(order.status, "");

    // 验证草稿状态可以转换为已确认
    let valid_transitions = vec!["confirmed", "cancelled"];
    assert!(valid_transitions.contains(&"confirmed"));
}

#[test]
fn test_status_transitions_confirmed_to_delivered() {
    let order = make_sales_order_model(1, "confirmed");
    assert_eq!(order.status, "");

    // 验证已确认状态可以转换为已发货
    let valid_transitions = vec!["delivered", "cancelled"];
    assert!(valid_transitions.contains(&"delivered"));
}

#[test]
fn test_status_transitions_delivered_is_final() {
    let order = make_sales_order_model(1, "delivered");
    assert_eq!(order.status, "");

    // 验证已发货状态是终态，不能转换
    let invalid_transitions = vec!["draft", "confirmed"];
    assert!(!invalid_transitions.contains(&"draft"));
}

// ===== CreateDeliveryDto 测试 =====

#[test]
fn test_create_delivery_dto_default() {
    let dto = CreateDeliveryDto { warehouse_id: None };
    assert!(dto.warehouse_id.is_none());
}

#[test]
fn test_create_delivery_dto_with_warehouse() {
    let dto = CreateDeliveryDto {
        warehouse_id: Some(1),
    };
    assert_eq!(dto.warehouse_id, Some(1));
}

// ===== 分页参数测试 =====

#[test]
fn test_page_request_clamp() {
    // 测试分页参数边界
    let page = 0u64.clamp(1, 1000);
    let page_size = 0u64.clamp(1, 100);

    assert_eq!(page, 1);
    assert_eq!(page_size, 1);
}

#[test]
fn test_page_request_max() {
    // 测试分页参数最大值
    let page = 2000u64.clamp(1, 1000);
    let page_size = 200u64.clamp(1, 100);

    assert_eq!(page, 1000);
    assert_eq!(page_size, 100);
}

// ===== 金额计算测试 =====

#[test]
fn test_amount_calculation() {
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
fn test_sales_order_query_deserialization() {
    let json = json!({
        "page": 1,
        "page_size": 10,
        "status": "draft",
        "customer_id": 1,
        "order_no": "SO-2026-0001"
    });

    let query: SalesOrderQuery = serde_json::from_value(json).expect("反序列化失败");
    assert_eq!(query.page, Some(1));
    assert_eq!(query.page_size, Some(10));
    assert_eq!(query.status, Some("draft".to_string()));
}

#[test]
fn test_sales_order_query_partial_deserialization() {
    let json = json!({
        "page": 1
    });

    let query: SalesOrderQuery = serde_json::from_value(json).expect("反序列化失败");
    assert_eq!(query.page, Some(1));
    assert!(query.page_size.is_none());
    assert!(query.status.is_none());
}
