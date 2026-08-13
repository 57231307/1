use bingxi_backend::services::rnd_super_deduction_service::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;

#[test]
fn test_calculate_super_deduction_75_percent() {
    let db = Arc::new(DatabaseConnection::default());
    let service = RndSuperDeductionService::new(db);
    let rd_expense = Decimal::from_str("1000000").unwrap();
    let rate = Decimal::from_str("0.75").unwrap();
    let result = service.calculate_super_deduction(rd_expense, rate);
    assert_eq!(result, Decimal::from_str("750000").unwrap());
}

#[test]
fn test_calculate_super_deduction_100_percent() {
    let db = Arc::new(DatabaseConnection::default());
    let service = RndSuperDeductionService::new(db);
    let rd_expense = Decimal::from_str("1000000").unwrap();
    let rate = Decimal::from_str("1.00").unwrap();
    let result = service.calculate_super_deduction(rd_expense, rate);
    assert_eq!(result, Decimal::from_str("1000000").unwrap());
}
