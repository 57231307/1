    use bingxi_backend::services::labor_contract_service::*;
    use bingxi_backend::services::pollution_permit_service::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_classify_expiry_level_normal() {
        assert_eq!(
            LaborContractService::classify_expiry_level(120),
            ContractExpiryLevel::Normal
        );
        assert_eq!(
            LaborContractService::classify_expiry_level(91),
            ContractExpiryLevel::Normal
        );
    }

    #[test]
    fn test_classify_expiry_level_30_days() {
        assert_eq!(
            LaborContractService::classify_expiry_level(30),
            ContractExpiryLevel::Warning30Days
        );
        assert_eq!(
            LaborContractService::classify_expiry_level(0),
            ContractExpiryLevel::Warning30Days
        );
    }

    #[test]
    fn test_classify_expiry_level_expired() {
        assert_eq!(
            LaborContractService::classify_expiry_level(-1),
            ContractExpiryLevel::Expired
        );
    }

    #[test]
    fn test_validate_probation_short_contract() {
        // 合同期 6 个月，试用期 1 个月（合法）
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let probation_end = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let result = LaborContractService::validate_probation(
            start,
            Some(end),
            probation_end,
            Decimal::new(4000, 0),
            Decimal::new(5000, 0),
        );
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_probation_too_long() {
        // 合同期 1 年，试用期 6 个月（违法，应 ≤ 2 个月）
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let probation_end = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let result = LaborContractService::validate_probation(
            start,
            Some(end),
            probation_end,
            Decimal::new(4000, 0),
            Decimal::new(5000, 0),
        );
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("试用期长度")));
    }

    #[test]
    fn test_validate_probation_salary_too_low() {
        // 试用期工资 < 转正工资 80%
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let probation_end = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let result = LaborContractService::validate_probation(
            start,
            Some(end),
            probation_end,
            Decimal::new(3000, 0), // 3000 < 5000 * 0.8 = 4000
            Decimal::new(5000, 0),
        );
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("80%")));
    }

    #[test]
    fn test_validate_contract_type_valid() {
        assert!(LaborContractService::validate_contract_type("fixed_term").is_ok());
        assert!(LaborContractService::validate_contract_type("permanent").is_ok());
        assert!(LaborContractService::validate_contract_type("task_based").is_ok());
    }

    #[test]
    fn test_validate_contract_type_invalid() {
        assert!(LaborContractService::validate_contract_type("invalid").is_err());
    }

    #[test]
    fn test_validate_working_hours_system_valid() {
        assert!(LaborContractService::validate_working_hours_system("standard").is_ok());
        assert!(LaborContractService::validate_working_hours_system("comprehensive").is_ok());
        assert!(LaborContractService::validate_working_hours_system("flexible").is_ok());
    }

    #[test]
    fn test_validate_working_hours_system_invalid() {
        assert!(LaborContractService::validate_working_hours_system("invalid").is_err());
    }
}