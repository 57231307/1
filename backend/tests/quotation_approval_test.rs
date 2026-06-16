//! 销售报价单审批服务测试
//!
//! Week 2 任务 7 - 销售报价单模块
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 7

use crate::services::quotation_approval_service::ApproverRole;
use rust_decimal::Decimal;

#[test]
fn test_approver_role_self_below_100k() {
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(50000)),
        ApproverRole::Salesperson
    );
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(99999)),
        ApproverRole::Salesperson
    );
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(0)),
        ApproverRole::Salesperson
    );
}

#[test]
fn test_approver_role_manager_100k_to_500k() {
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(100000)),
        ApproverRole::SalesManager
    );
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(300000)),
        ApproverRole::SalesManager
    );
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(499999)),
        ApproverRole::SalesManager
    );
}

#[test]
fn test_approver_role_general_manager_above_500k() {
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(500000)),
        ApproverRole::GeneralManager
    );
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(1_000_000)),
        ApproverRole::GeneralManager
    );
    assert_eq!(
        ApproverRole::from_amount(Decimal::from(10_000_000)),
        ApproverRole::GeneralManager
    );
}

#[test]
fn test_approver_role_code_mapping() {
    assert_eq!(ApproverRole::Salesperson.code(), "self");
    assert_eq!(ApproverRole::SalesManager.code(), "sales_manager");
    assert_eq!(ApproverRole::GeneralManager.code(), "general_manager");
}

#[test]
fn test_approver_role_partial_eq() {
    assert_eq!(ApproverRole::Salesperson, ApproverRole::Salesperson);
    assert_ne!(ApproverRole::Salesperson, ApproverRole::SalesManager);
    assert_ne!(ApproverRole::SalesManager, ApproverRole::GeneralManager);
}
