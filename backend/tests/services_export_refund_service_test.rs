use rust_decimal::Decimal;
use bingxi_backend::services::export_refund_service::*;

#[test]
fn test_calculate_exempt_credit_refund_normal() {
    let input = RefundCalculationInput {
        export_sales_amount: Decimal::new(1000000, 0), // 100万
        refund_rate: Decimal::new(13, 2),              // 13%
        input_vat_amount: Decimal::new(80000, 0),      // 8万
        carryforward_from_prev: Decimal::ZERO,
    };
    let result = ExportRefundService::calculate_exempt_credit_refund(&input);
    // 免抵退税额 = 100万 × 13% = 13万
    assert_eq!(result.refundable_vat_amount, Decimal::new(130000, 0));
    // 应退税额 = min(13万, 8万) = 8万
    assert_eq!(result.actual_refund_amount, Decimal::new(80000, 0));
    // 免抵税额 = 13万 - 8万 = 5万
    assert_eq!(result.exempt_vat_amount, Decimal::new(50000, 0));
    // 结转下期 = 0
    assert_eq!(result.carryforward_amount, Decimal::ZERO);
}

#[test]
fn test_calculate_exempt_credit_refund_with_carryforward() {
    let input = RefundCalculationInput {
        export_sales_amount: Decimal::new(100000, 0),   // 10万
        refund_rate: Decimal::new(13, 2),               // 13%
        input_vat_amount: Decimal::new(50000, 0),       // 5万
        carryforward_from_prev: Decimal::new(30000, 0), // 3万
    };
    let result = ExportRefundService::calculate_exempt_credit_refund(&input);
    // 免抵退税额 = 10万 × 13% = 1.3万
    assert_eq!(result.refundable_vat_amount, Decimal::new(13000, 0));
    // 当期可抵扣 = 3万 + 5万 = 8万
    // 应退税额 = min(1.3万, 8万) = 1.3万
    assert_eq!(result.actual_refund_amount, Decimal::new(13000, 0));
    // 免抵税额 = 1.3万 - 1.3万 = 0
    assert_eq!(result.exempt_vat_amount, Decimal::ZERO);
    // 结转下期 = 8万 - 1.3万 = 6.7万
    assert_eq!(result.carryforward_amount, Decimal::new(67000, 0));
}