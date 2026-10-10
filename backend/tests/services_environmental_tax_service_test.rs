use bingxi_backend::services::environmental_tax_service::*;
use rust_decimal::Decimal;

/// 测试注入的适用税额（模拟部署配置值；**不是生产默认值**——生产无默认税额，
/// 未配置时计税显式失败，见 contract_wave5_env_tax_rates_from_config_test.rs）
fn injected_tax_rate() -> Option<Decimal> {
    Some(Decimal::new(24, 1)) // 2.4（仅测试夹具取值）
}

#[test]
fn test_calculate_tax_cod() {
    let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
        "wastewater",
        "COD",
        Decimal::new(100, 0), // 100kg
        None,
        injected_tax_rate(),
    )
    .expect("COD 已登记法定当量值且税额已配置，计税应成功");
    // 污染当量数 = 100 / 1 = 100
    assert_eq!(equivalent, Decimal::new(100, 0));
    // 应缴税额 = 100 × 2.4（注入值）= 240
    assert_eq!(tax, Decimal::new(240, 0));
}

#[test]
fn test_calculate_tax_vocs() {
    let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
        "exhaust",
        "VOCs",
        Decimal::new(50, 0), // 50kg
        None,
        injected_tax_rate(),
    )
    .expect("VOCs 已登记法定当量值且税额已配置，计税应成功");
    // 污染当量数 = 50 / 0.5 = 100
    assert_eq!(equivalent, Decimal::new(100, 0));
    // 应缴税额 = 100 × 2.4（注入值）= 240
    assert_eq!(tax, Decimal::new(240, 0));
}

#[test]
fn test_validate_discharge_type_valid() {
    assert!(EnvironmentalTaxService::validate_discharge_type("wastewater").is_ok());
    assert!(EnvironmentalTaxService::validate_discharge_type("exhaust").is_ok());
    assert!(EnvironmentalTaxService::validate_discharge_type("solid_waste").is_ok());
}

#[test]
fn test_validate_discharge_type_invalid() {
    assert!(EnvironmentalTaxService::validate_discharge_type("invalid").is_err());
}
