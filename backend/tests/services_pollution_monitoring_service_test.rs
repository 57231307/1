#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_check_exceedance_normal() {
        let (is_exceeding, ratio) =
            PollutionMonitoringService::check_exceedance(Decimal::new(50, 0), Decimal::new(80, 0));
        assert!(!is_exceeding);
        assert_eq!(ratio, None);
    }

    #[test]
    fn test_check_exceedance_at_limit() {
        // 实测值等于限值不算超标
        let (is_exceeding, ratio) =
            PollutionMonitoringService::check_exceedance(Decimal::new(80, 0), Decimal::new(80, 0));
        assert!(!is_exceeding);
        assert_eq!(ratio, None);
    }

    #[test]
    fn test_check_exceedance_exceeded() {
        // 实测 120，限值 80 → 超标 0.5 倍
        let (is_exceeding, ratio) =
            PollutionMonitoringService::check_exceedance(Decimal::new(120, 0), Decimal::new(80, 0));
        assert!(is_exceeding);
        assert_eq!(ratio, Some(Decimal::new(5, 1))); // 0.5
    }

    #[test]
    fn test_validate_monitoring_type() {
        assert!(PollutionMonitoringService::validate_monitoring_type("wastewater").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("exhaust").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("noise").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("solid_waste").is_ok());
        assert!(PollutionMonitoringService::validate_monitoring_type("invalid").is_err());
    }

    #[test]
    fn test_validate_waste_type() {
        assert!(PollutionMonitoringService::validate_waste_type("sludge").is_ok());
        assert!(PollutionMonitoringService::validate_waste_type("waste_fabric").is_ok());
        assert!(PollutionMonitoringService::validate_waste_type("chemical_waste").is_ok());
        assert!(PollutionMonitoringService::validate_waste_type("invalid").is_err());
    }

    #[test]
    fn test_validate_disposal_method() {
        assert!(PollutionMonitoringService::validate_disposal_method("landfill").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("incineration").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("reuse").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("storage").is_ok());
        assert!(PollutionMonitoringService::validate_disposal_method("invalid").is_err());
    }

    #[test]
    fn test_pollution_limit_reference_cod() {
        let limit = PollutionLimitReference::get_limit("wastewater", "COD");
        assert_eq!(limit, Some(Decimal::new(80, 0)));
    }

    #[test]
    fn test_pollution_limit_reference_vocs() {
        let limit = PollutionLimitReference::get_limit("exhaust", "VOCs");
        assert_eq!(limit, Some(Decimal::new(60, 0)));
    }

    #[test]
    fn test_pollution_limit_reference_unknown() {
        let limit = PollutionLimitReference::get_limit("wastewater", "Unknown");
        assert_eq!(limit, None);
    }
}