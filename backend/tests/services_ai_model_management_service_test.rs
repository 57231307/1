    use bingxi_backend::services::ai_model_management_service::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_validate_model_status_valid() {
        assert!(AiModelManagementService::validate_model_status("draft").is_ok());
        assert!(AiModelManagementService::validate_model_status("active").is_ok());
        assert!(AiModelManagementService::validate_model_status("retired").is_ok());
        assert!(AiModelManagementService::validate_model_status("archived").is_ok());
    }

    #[test]
    fn test_validate_model_status_invalid() {
        assert!(AiModelManagementService::validate_model_status("invalid").is_err());
        assert!(AiModelManagementService::validate_model_status("").is_err());
    }

    #[test]
    fn test_validate_approval_status_valid() {
        assert!(AiModelManagementService::validate_approval_status("pending").is_ok());
        assert!(AiModelManagementService::validate_approval_status("approved").is_ok());
        assert!(AiModelManagementService::validate_approval_status("rejected").is_ok());
    }

    #[test]
    fn test_validate_decision_type_valid() {
        assert!(AiModelManagementService::validate_decision_type("process_optimization").is_ok());
        assert!(AiModelManagementService::validate_decision_type("quality_prediction").is_ok());
        assert!(AiModelManagementService::validate_decision_type("sales_forecast").is_ok());
    }

    #[test]
    fn test_validate_decision_type_invalid() {
        assert!(AiModelManagementService::validate_decision_type("invalid_type").is_err());
    }

    #[test]
    fn test_normalize_risk() {
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("high"),
            "high"
        );
        assert_eq!(AiQualityReconciliationService::normalize_risk("高"), "high");
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("medium"),
            "medium"
        );
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("中"),
            "medium"
        );
        assert_eq!(AiQualityReconciliationService::normalize_risk("low"), "low");
        assert_eq!(AiQualityReconciliationService::normalize_risk("低"), "low");
        assert_eq!(
            AiQualityReconciliationService::normalize_risk("unknown_val"),
            "unknown"
        );
    }

    #[test]
    fn test_validate_metric_range() {
        let svc = AiModelManagementService::new(std::sync::Arc::new(
            sea_orm::DatabaseConnection::default(),
        ));
        let _ = svc; // 抑制 unused 警告
        assert!(AiModelManagementService::validate_metric_range(
            "accuracy",
            Some(Decimal::new(85, 2))
        )
        .is_ok());
        assert!(AiModelManagementService::validate_metric_range("accuracy", None).is_ok());
        assert!(AiModelManagementService::validate_metric_range(
            "accuracy",
            Some(Decimal::new(150, 2))
        )
        .is_err());
        assert!(AiModelManagementService::validate_metric_range(
            "accuracy",
            Some(Decimal::new(-5, 2))
        )
        .is_err());
    }
}