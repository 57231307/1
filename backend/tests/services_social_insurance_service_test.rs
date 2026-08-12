#[cfg(test)]
mod tests {
use bingxi_backend::services::social_insurance_service::*;


    #[test]
    fn test_calculate_insurance_default_rates() {
        let config = InsuranceRateConfig::default();
        let result = SocialInsuranceService::calculate_insurance(Decimal::new(10000, 0), &config);

        // 养老保险：单位 1600 + 个人 800
        assert_eq!(result.pension_employer, Decimal::new(1600, 0));
        assert_eq!(result.pension_employee, Decimal::new(800, 0));

        // 医疗保险：单位 800 + 个人 200
        assert_eq!(result.medical_employer, Decimal::new(800, 0));
        assert_eq!(result.medical_employee, Decimal::new(200, 0));

        // 公积金：单位 1200 + 个人 1200
        assert_eq!(result.housing_fund_employer, Decimal::new(1200, 0));
        assert_eq!(result.housing_fund_employee, Decimal::new(1200, 0));

        // 单位合计：1600+800+50+40+100+1200 = 3790
        assert_eq!(result.total_employer, Decimal::new(3790, 0));

        // 个人合计：800+200+50+1200 = 2250
        assert_eq!(result.total_employee, Decimal::new(2250, 0));
    }

    #[test]
    fn test_validate_base_amount_normal() {
        // validate_base_amount 是纯函数，不依赖数据库连接
        let validation =
            SocialInsuranceService::validate_base_amount_static(Decimal::new(10000, 0));
        assert!(validation.is_valid);
        assert!(!validation.is_below_minimum);
        assert!(!validation.is_above_maximum);
    }

    #[test]
    fn test_validate_period_valid() {
        assert!(SocialInsuranceService::validate_period(2026, 1).is_ok());
        assert!(SocialInsuranceService::validate_period(2026, 12).is_ok());
    }

    #[test]
    fn test_validate_period_invalid_year() {
        assert!(SocialInsuranceService::validate_period(1999, 1).is_err());
        assert!(SocialInsuranceService::validate_period(2101, 1).is_err());
    }

    #[test]
    fn test_validate_period_invalid_month() {
        assert!(SocialInsuranceService::validate_period(2026, 0).is_err());
        assert!(SocialInsuranceService::validate_period(2026, 13).is_err());
    }
}