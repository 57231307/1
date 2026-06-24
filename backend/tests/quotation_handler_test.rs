//! 销售报价单 Handler 集成测试
//!
//! Week 2 任务 9 - 销售报价单模块
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 9
//!
//! 注：完整 HTTP 集成测试需要启动 AppState / 路由，
//! 本测试仅覆盖 DTO/请求体的反序列化和校验逻辑。

use bingxi_backend::models::quotation_response_dto::QuotationResponseDto;
use bingxi_backend::services::quotation_pricing_service::{CustomerLevel, PricingContext};

#[test]
fn test_reject_request_serde() {
    use bingxi_backend::handlers::quotation_handler::RejectRequest;
    let json = r#"{"reason":"价格过高"}"#;
    let req: RejectRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.reason, "价格过高");
}

#[test]
fn test_pricing_context_deserialize() {
    let json = r#"{
        "customer_id": 1,
        "customer_level": "VIP",
        "product_id": 10,
        "color_id": 5,
        "quantity": 100,
        "currency": "CNY",
        "quotation_date": "2026-06-16"
    }"#;
    let ctx: PricingContext = serde_json::from_str(json).unwrap();
    assert_eq!(ctx.customer_id, 1);
    assert_eq!(ctx.customer_level, CustomerLevel::VIP);
    assert_eq!(ctx.quantity.to_string(), "100");
    assert_eq!(ctx.currency, "CNY");
}

#[test]
fn test_quotation_response_dto_default() {
    let dto = QuotationResponseDto::default();
    assert_eq!(dto.id, 0);
    assert!(dto.items.is_empty());
    assert!(dto.terms.is_empty());
}

#[test]
fn test_expiry_query_default() {
    use bingxi_backend::handlers::quotation_handler::ExpiryQuery;
    let q = ExpiryQuery::default();
    assert!(q.days.is_none());
}

#[test]
fn test_color_price_upsert_request() {
    use bingxi_backend::handlers::quotation_handler::ColorPriceUpsertRequest;
    let json = r#"{
        "id": null,
        "product_id": 1,
        "color_id": 5,
        "currency": "CNY",
        "base_price": 100.5,
        "effective_from": "2026-01-01",
        "effective_to": "2026-12-31",
        "customer_level": "VIP",
        "min_quantity": 10,
        "notes": "测试色号价格"
    }"#;
    let req: ColorPriceUpsertRequest = serde_json::from_str(json).unwrap();
    assert!(req.id.is_none());
    assert_eq!(req.product_id, 1);
    assert_eq!(req.base_price.to_string(), "100.5");
    assert_eq!(req.customer_level.as_deref(), Some("VIP"));
}
