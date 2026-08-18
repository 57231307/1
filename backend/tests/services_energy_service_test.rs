use bingxi_backend::services::energy_service::*;
use bingxi_backend::services::outsourcing_service::*;
use rust_decimal::Decimal;

#[test]
fn 测试计算消耗量_正常() {
    let result = compute_consumption(Decimal::new(100, 0), Decimal::new(150, 0));
    assert_eq!(result, Decimal::new(50, 0));
}

#[test]
fn 测试计算消耗量_当前小于上次返回零() {
    let result = compute_consumption(Decimal::new(150, 0), Decimal::new(100, 0));
    assert_eq!(result, Decimal::ZERO);
}

#[test]
fn 测试计算消耗量_相等返回零() {
    let result = compute_consumption(Decimal::new(100, 0), Decimal::new(100, 0));
    assert_eq!(result, Decimal::ZERO);
}

#[test]
fn 测试计算总成本() {
    let result = compute_total_cost(Decimal::new(50, 0), Decimal::new(12, 1));
    assert_eq!(result, Decimal::new(60, 0));
}

#[test]
fn 测试计算分摊比例_正常() {
    let result = compute_allocation_ratio(Decimal::new(30, 0), Decimal::new(100, 0));
    assert_eq!(result, Decimal::new(30, 2)); // 0.30
}

#[test]
fn 测试计算分摊比例_总依据为零返回零() {
    let result = compute_allocation_ratio(Decimal::new(30, 0), Decimal::ZERO);
    assert_eq!(result, Decimal::ZERO);
}

#[test]
fn 测试计算分摊消耗量() {
    let result = compute_allocated_consumption(
        Decimal::new(1000, 0),
        Decimal::new(30, 2), // 0.30
    );
    assert_eq!(result, Decimal::new(300, 0));
}

#[test]
fn 测试计算分摊成本() {
    let result = compute_allocated_cost(
        Decimal::new(5000, 0),
        Decimal::new(30, 2), // 0.30
    );
    assert_eq!(result, Decimal::new(1500, 0));
}

#[test]
fn 测试计算单位能耗_正常() {
    let result = compute_unit_consumption(Decimal::new(300, 0), Some(Decimal::new(100, 0)));
    assert_eq!(result, Some(Decimal::new(3, 0)));
}

#[test]
fn test_unit_energy_zero_output_returns_none() {
    let result = compute_unit_consumption(Decimal::new(300, 0), Some(Decimal::ZERO));
    assert_eq!(result, None);
}

#[test]
fn test_unit_energy_none_output_returns_none() {
    let result = compute_unit_consumption(Decimal::new(300, 0), None);
    assert_eq!(result, None);
}

#[test]
fn 测试超基准判断_正常未超() {
    let (exceeds, deviation) = check_consumption_exceeds_standard(
        Decimal::new(95, 0),
        Decimal::new(100, 0),
        Decimal::new(10, 2), // 0.10
    );
    assert!(!exceeds);
    assert_eq!(deviation, Decimal::ZERO);
}

#[test]
fn 测试超基准判断_超出阈值() {
    // 标准 100，容差 10%，阈值 110，实际 120 → 超基准
    let (exceeds, deviation) = check_consumption_exceeds_standard(
        Decimal::new(120, 0),
        Decimal::new(100, 0),
        Decimal::new(10, 2),
    );
    assert!(exceeds);
    // 偏差 = (120 - 100) / 100 × 100 = 20
    assert_eq!(deviation, Decimal::new(20, 0));
}

#[test]
fn 测试超基准判断_标准为零返回未超() {
    let (exceeds, deviation) = check_consumption_exceeds_standard(
        Decimal::new(120, 0),
        Decimal::ZERO,
        Decimal::new(10, 2),
    );
    assert!(!exceeds);
    assert_eq!(deviation, Decimal::ZERO);
}

#[test]
fn 测试校验能源类型_合法() {
    assert!(validate_meter_type("water").is_ok());
    assert!(validate_meter_type("electricity").is_ok());
    assert!(validate_meter_type("steam").is_ok());
    assert!(validate_meter_type("gas").is_ok());
    assert!(validate_meter_type("compressed_air").is_ok());
}

#[test]
fn 测试校验能源类型_非法() {
    assert!(validate_meter_type("invalid").is_err());
}

#[test]
fn 测试校验分摊基准_合法() {
    assert!(validate_allocation_basis("by_duration").is_ok());
    assert!(validate_allocation_basis("by_output").is_ok());
    assert!(validate_allocation_basis("by_equipment").is_ok());
    assert!(validate_allocation_basis("by_workshop").is_ok());
}

#[test]
fn 测试校验分摊基准_非法() {
    assert!(validate_allocation_basis("invalid").is_err());
}
