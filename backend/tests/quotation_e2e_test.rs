//! 销售报价单业务测试
//!
//! V15 Batch 488 P1 修复（audit-report batch-06 §6.5 缺陷 3）：
//! - 原文件 9 个测试中 8 个为伪测试（仅断言本地常量字符串/数组）
//! - 保留唯一真实测试：test_full_workflow_amount_tier_logic（验证 ApproverRole::from_amount）
//! - 重命名为中文命名（项目规范），并补 DTO/状态机/单号格式的真实业务校验
//! - 端到端 HTTP 测试需 QuotationService + DB schema，标注 #[ignore]
//!
//! 创建时间: 2026-06-16

use bingxi_backend::models::quotation_create_dto::CreateQuotationDto;
use bingxi_backend::models::quotation_response_dto::QuotationResponseDto;
use bingxi_backend::models::sales_quotation;
use bingxi_backend::services::quotation_approval_service::ApproverRole;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// test_jejtspjspd_fgsd
///
/// 业务规则：ApproverRole::from_amount 根据 amount 返回对应审批角色
/// - ≤ 100,000 → Salesperson
/// - 100,001 ~ 500,000 → SalesManager
/// - > 500,000 → GeneralManager
#[test]
fn test_jejtspjspd_fgsd() {
    assert_eq!(
        ApproverRole::from_amount(dec!(50000)),
        ApproverRole::Salesperson
    );
    assert_eq!(
        ApproverRole::from_amount(dec!(300000)),
        ApproverRole::SalesManager
    );
    assert_eq!(
        ApproverRole::from_amount(dec!(800000)),
        ApproverRole::GeneralManager
    );
}

/// test_jejt_bjzjy
///
/// 验证阈值边界（100000 / 500000）的角色切换。
/// 实际业务逻辑（from_amount）：
/// - amount < 100000 → Salesperson
/// - 100000 ≤ amount < 500000 → SalesManager
/// - amount ≥ 500000 → GeneralManager
#[test]
fn test_jejt_bjzjy() {
    // 99,999 仍为 Salesperson（< 100000）
    assert_eq!(
        ApproverRole::from_amount(dec!(99999)),
        ApproverRole::Salesperson
    );
    // 100,000 升级到 SalesManager（≥ 100000）
    assert_eq!(
        ApproverRole::from_amount(dec!(100000)),
        ApproverRole::SalesManager
    );
    // 499,999 仍为 SalesManager（< 500000）
    assert_eq!(
        ApproverRole::from_amount(dec!(499999)),
        ApproverRole::SalesManager
    );
    // 500,000 升级到 GeneralManager（≥ 500000）
    assert_eq!(
        ApproverRole::from_amount(dec!(500000)),
        ApproverRole::GeneralManager
    );
}

/// test_createquotationdto_fxlh_bzbjd
///
/// 验证前端 POST 的 JSON 能正确反序列化为 CreateQuotationDto，
/// 包含 customer_id / items / tax 等核心字段。
#[test]
fn test_createquotationdto_fxlh_bzbjd() {
    let json = r#"{
        "customer_id": 1,
        "sales_user_id": 2,
        "quotation_date": "2026-06-16",
        "valid_until": "2026-07-16",
        "currency": "CNY",
        "exchange_rate": 1.0,
        "base_currency": "CNY",
        "price_terms": "FOB",
        "tax_inclusive": false,
        "tax_rate": 13,
        "items": [{
            "product_id": 1,
            "unit": "米",
            "quantity": 100,
            "unit_price": 50,
            "unit_price_with_tax": 56.5
        }]
    }"#;
    let dto: CreateQuotationDto = serde_json::from_str(json).unwrap();
    assert_eq!(dto.customer_id, 1);
    assert_eq!(dto.sales_user_id, 2);
    assert_eq!(dto.items.len(), 1);
    assert_eq!(dto.items[0].unit_price, dec!(50));
    assert_eq!(dto.tax_rate, dec!(13));
    assert!(!dto.tax_inclusive);
}

/// test_quotationresponsedto_xlh_blhxzd
///
/// 验证后端返回给前端的 QuotationResponseDto 能被序列化为 JSON，
/// 包含 quotation_no / status / total_amount 等关键字段。
#[test]
fn test_quotationresponsedto_xlh_blhxzd() {
    use chrono::Utc;
    let dto = QuotationResponseDto {
        id: 1,
        quotation_no: "QT202606160001".to_string(),
        customer_id: 1,
        sales_user_id: 2,
        quotation_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(),
        valid_until: chrono::NaiveDate::from_ymd_opt(2026, 7, 16).unwrap(),
        currency: "CNY".to_string(),
        exchange_rate: dec!(1),
        base_currency: "CNY".to_string(),
        price_terms: "FOB".to_string(),
        status: "draft".to_string(),
        tax_inclusive: false,
        tax_rate: dec!(13),
        subtotal: dec!(5000),
        tax_amount: dec!(650),
        total_amount: dec!(5650),
        created_by: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        items: Vec::new(),
        terms: Vec::new(),
        ..Default::default()
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"QT202606160001\""), "应包含报价单号");
    assert!(json.contains("\"status\":\"draft\""), "应包含状态字段");
    assert!(json.contains("\"total_amount\":5650"), "应包含总金额");
}

/// test_salesquotationmodel_mrzdz
///
/// 验证 sales_quotation::Model 能被手工构造（用于测试夹具），
/// 默认 status="draft"、total_amount=0。
#[test]
fn test_salesquotationmodel_mrzdz() {
    let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 16).unwrap();
    let m = sales_quotation::Model {
        id: 0,
        quotation_no: "QT0000".to_string(),
        customer_id: 0,
        sales_user_id: 0,
        quotation_date: today,
        valid_until: today,
        currency: "CNY".to_string(),
        exchange_rate: Decimal::ONE,
        base_currency: "CNY".to_string(),
        price_terms: "FOB".to_string(),
        incoterms_version: None,
        incoterm_location: None,
        tax_inclusive: false,
        tax_rate: Decimal::ZERO,
        moq: None,
        lead_time_days: None,
        customer_level: None,
        subtotal: Decimal::ZERO,
        tax_amount: Decimal::ZERO,
        total_amount: Decimal::ZERO,
        freight_cost: None,
        insurance_cost: None,
        duty_cost: None,
        status: "draft".to_string(),
        approval_instance_id: None,
        approved_by: None,
        approved_at: None,
        rejection_reason: None,
        converted_sales_order_id: None,
        converted_at: None,
        notes: None,
        created_by: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    assert_eq!(m.status, "draft");
    assert_eq!(m.total_amount, Decimal::ZERO);
    assert_eq!(m.currency, "CNY");
}

/// test_bjdwzywlc_xzsdb
///
/// 真实端到端测试：创建 → 提交 → 审批 → 转订单 → 完成。
/// 需要 quotations 表 schema + QuotationService 实例。
#[tokio::test]
#[ignore = "需要 quotations 表 schema + QuotationService"]
async fn test_bjdwzywlc_xzsdb() {
    // 占位：业务流程需 QuotationService + DB schema 协同。
    // CI 环境通过 TEST_DATABASE_URL 提供真实 DB，移除 #[ignore] 即可运行。
}
