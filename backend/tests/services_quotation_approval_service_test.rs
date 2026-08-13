use bingxi_backend::services::quotation_approval_service::*;
use rust_decimal_macros::dec;

#[test]
fn test_approver_role_small_amount_is_salesperson() {
    assert_eq!(
        ApproverRole::from_amount(dec!(50000)),
        ApproverRole::Salesperson
    );
    assert_eq!(
        ApproverRole::from_amount(dec!(99999)),
        ApproverRole::Salesperson
    );
}

#[test]
fn test_approver_role_medium_amount_is_sales_manager() {
    assert_eq!(
        ApproverRole::from_amount(dec!(100000)),
        ApproverRole::SalesManager
    );
    assert_eq!(
        ApproverRole::from_amount(dec!(300000)),
        ApproverRole::SalesManager
    );
    assert_eq!(
        ApproverRole::from_amount(dec!(499999)),
        ApproverRole::SalesManager
    );
}

#[test]
fn test_approver_role_large_amount_is_general_manager() {
    assert_eq!(
        ApproverRole::from_amount(dec!(500000)),
        ApproverRole::GeneralManager
    );
    assert_eq!(
        ApproverRole::from_amount(dec!(1000000)),
        ApproverRole::GeneralManager
    );
}

#[test]
fn test_approver_role_code() {
    assert_eq!(ApproverRole::Salesperson.code(), "self");
    assert_eq!(ApproverRole::SalesManager.code(), "sales_manager");
    assert_eq!(ApproverRole::GeneralManager.code(), "general_manager");
}
