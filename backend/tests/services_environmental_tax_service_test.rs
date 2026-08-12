    use bingxi_backend::services::environmental_tax_service::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_calculate_tax_cod() {
        let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
            "wastewater",
            "COD",
            Decimal::new(100, 0), // 100kg
            None,
        );
        // 污染当量数 = 100 / 1 = 100
        assert_eq!(equivalent, Decimal::new(100, 0));
        // 应缴税额 = 100 × 2.4 = 240
        assert_eq!(tax, Decimal::new(240, 0));
    }

    #[test]
    fn test_calculate_tax_vocs() {
        let (equivalent, tax) = EnvironmentalTaxService::calculate_tax(
            "exhaust",
            "VOCs",
            Decimal::new(50, 0), // 50kg
            None,
        );
        // 污染当量数 = 50 / 0.5 = 100
        assert_eq!(equivalent, Decimal::new(100, 0));
        // 应缴税额 = 100 × 2.4 = 240
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
}