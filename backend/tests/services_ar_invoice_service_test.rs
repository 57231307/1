// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::models::status::payment::{PAYMENT_PAID, PAYMENT_PARTIAL_PAID};
use rust_decimal::Decimal;

// ========== derive_paid_status 纯函数测试 ==========

/// test_tdfkzt_ysjedyfpje_fhpaid（验证 received > invoice 时返回 PAID（已收齐）。）
#[test]
fn test_tdfkzt_ysjedyfpje_fhpaid() {
    let received = decs!("123.45");
    let invoice = decs!("100.00");
    assert_eq!(
        ArInvoiceService::derive_paid_status(received, invoice),
        PAYMENT_PAID
    );
}

/// test_tdfkzt_ysjedyfpje_bjfhpaid（验证 received == invoice 边界场景返回 PAID（已收齐）。）
#[test]
fn test_tdfkzt_ysjedyfpje_bjfhpaid() {
    let received = decs!("100.00");
    let invoice = decs!("100.00");
    assert_eq!(
        ArInvoiceService::derive_paid_status(received, invoice),
        PAYMENT_PAID
    );
}

/// test_tdfkzt_ysjewl_fhpartial_paid（验证 received == 0（发票金额非零）时返回 PARTIAL_PAID（部分收款）。）
#[test]
fn test_tdfkzt_ysjewl_fhpartial_paid() {
    let received = Decimal::ZERO;
    let invoice = decs!("100.00");
    assert_eq!(
        ArInvoiceService::derive_paid_status(received, invoice),
        PAYMENT_PARTIAL_PAID
    );
}

/// test_tdfkzt_ysjexyfpje_fhpartial_paid（验证 0 < received < invoice 时返回 PARTIAL_PAID（部分收款）。）
#[test]
fn test_tdfkzt_ysjexyfpje_fhpartial_paid() {
    let received = decs!("30.00");
    let invoice = decs!("100.00");
    assert_eq!(
        ArInvoiceService::derive_paid_status(received, invoice),
        PAYMENT_PARTIAL_PAID
    );
}

/// test_tdfkzt_ysjehfpjejwl_bjfhpaid（验证 received == 0 且 invoice == 0 边界场景：0 >= 0 为真，返回 PAID。）
#[test]
fn test_tdfkzt_ysjehfpjejwl_bjfhpaid() {
    let received = Decimal::ZERO;
    let invoice = Decimal::ZERO;
    assert_eq!(
        ArInvoiceService::derive_paid_status(received, invoice),
        PAYMENT_PAID
    );
}
