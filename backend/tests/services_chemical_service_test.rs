use chrono::NaiveDate;
use rust_decimal::Decimal;
use bingxi_backend::handlers::inventory_stock_handler::*;
use bingxi_backend::services::chemical_service::*;
use bingxi_backend::services::outsourcing_service::*;

#[test]
fn 测试计算剩余保质期_未过期() {
    let expiry = NaiveDate::from_ymd_opt(2025, 12, 31);
    let today = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let result = compute_remaining_shelf_life(expiry, today);
    assert_eq!(result, Some(364));
}

#[test]
fn 测试计算剩余保质期_已过期返回负数() {
    let expiry = NaiveDate::from_ymd_opt(2025, 1, 1);
    let today = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    let result = compute_remaining_shelf_life(expiry, today);
    assert_eq!(result, Some(-364));
}

#[test]
fn test_shelf_life_no_expiry_returns_none() {
    let today = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let result = compute_remaining_shelf_life(None, today);
    assert_eq!(result, None);
}

#[test]
fn 测试计算总成本() {
    let result = compute_total_cost(Decimal::new(100, 0), Decimal::new(12, 1));
    assert_eq!(result, Decimal::new(1200, 0));
}

#[test]
fn 测试计算总成本_零数量() {
    let result = compute_total_cost(Decimal::ZERO, Decimal::new(12, 1));
    assert_eq!(result, Decimal::ZERO);
}

#[test]
fn 测试校验染化料类型_合法() {
    assert!(validate_chemical_type("dye").is_ok());
    assert!(validate_chemical_type("auxiliary").is_ok());
    assert!(validate_chemical_type("chemical").is_ok());
}

#[test]
fn 测试校验染化料类型_非法() {
    assert!(validate_chemical_type("invalid").is_err());
}

#[test]
fn 测试校验来料检验状态_合法() {
    assert!(validate_inspection_status("pending").is_ok());
    assert!(validate_inspection_status("passed").is_ok());
    assert!(validate_inspection_status("failed").is_ok());
    assert!(validate_inspection_status("quarantine").is_ok());
}

#[test]
fn 测试校验来料检验状态_非法() {
    assert!(validate_inspection_status("invalid").is_err());
}

#[test]
fn 测试校验批次状态_合法() {
    assert!(validate_lot_status("active").is_ok());
    assert!(validate_lot_status("consumed").is_ok());
    assert!(validate_lot_status("expired").is_ok());
    assert!(validate_lot_status("scrapped").is_ok());
}

#[test]
fn 测试校验批次状态_非法() {
    assert!(validate_lot_status("invalid").is_err());
}

#[test]
fn 测试校验领用单类型_合法() {
    assert!(validate_requisition_type("production").is_ok());
    assert!(validate_requisition_type("lab").is_ok());
    assert!(validate_requisition_type("rd").is_ok());
}

#[test]
fn 测试校验领用单类型_非法() {
    assert!(validate_requisition_type("invalid").is_err());
}

#[test]
fn 测试校验领用单状态_合法() {
    assert!(validate_requisition_status("draft").is_ok());
    assert!(validate_requisition_status("approved").is_ok());
    assert!(validate_requisition_status("issued").is_ok());
    assert!(validate_requisition_status("partial_returned").is_ok());
    assert!(validate_requisition_status("closed").is_ok());
    assert!(validate_requisition_status("cancelled").is_ok());
}

#[test]
fn 测试校验领用单状态_非法() {
    assert!(validate_requisition_status("invalid").is_err());
}

#[test]
fn 测试低库存检查_低于安全库存() {
    let (below_safety, below_reorder) =
        check_low_stock(Decimal::new(5, 0), Decimal::new(10, 0), Decimal::new(20, 0));
    assert!(below_safety);
    assert!(below_reorder);
}

#[test]
fn 测试低库存检查_低于再订货点但高于安全库存() {
    let (below_safety, below_reorder) = check_low_stock(
        Decimal::new(15, 0),
        Decimal::new(10, 0),
        Decimal::new(20, 0),
    );
    assert!(!below_safety);
    assert!(below_reorder);
}

#[test]
fn 测试低库存检查_正常库存() {
    let (below_safety, below_reorder) = check_low_stock(
        Decimal::new(50, 0),
        Decimal::new(10, 0),
        Decimal::new(20, 0),
    );
    assert!(!below_safety);
    assert!(!below_reorder);
}