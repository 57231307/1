    use super::*;
#[cfg(test)]
mod tests {

    /// 测试风险等级评估
    #[test]
    fn test_risk_level_from_row_count() {
        assert_eq!(RiskLevel::from_row_count(0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_row_count(999), RiskLevel::Low);
        assert_eq!(RiskLevel::from_row_count(1000), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_row_count(9999), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_row_count(10000), RiskLevel::High);
        assert_eq!(RiskLevel::from_row_count(49999), RiskLevel::High);
        assert_eq!(RiskLevel::from_row_count(50000), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_row_count(100000), RiskLevel::Critical);
    }

    /// 测试审批状态序列化
    #[test]
    fn test_approval_status_as_str() {
        assert_eq!(ApprovalStatus::Pending.as_str(), "pending");
        assert_eq!(ApprovalStatus::PendingL2.as_str(), "pending_l2");
        assert_eq!(ApprovalStatus::Approved.as_str(), "approved");
        assert_eq!(ApprovalStatus::Rejected.as_str(), "rejected");
        assert_eq!(ApprovalStatus::Expired.as_str(), "expired");
        assert_eq!(ApprovalStatus::Cancelled.as_str(), "cancelled");
    }

    /// 测试审批状态解析
    #[test]
    fn test_approval_status_parse_status() {
        assert_eq!(
            ApprovalStatus::parse_status("pending"),
            Some(ApprovalStatus::Pending)
        );
        assert_eq!(
            ApprovalStatus::parse_status("pending_l2"),
            Some(ApprovalStatus::PendingL2)
        );
        assert_eq!(
            ApprovalStatus::parse_status("approved"),
            Some(ApprovalStatus::Approved)
        );
        assert_eq!(ApprovalStatus::parse_status("unknown"), None);
    }

    /// 测试敏感资源判断
    #[test]
    fn test_sensitive_resources() {
        assert!(sensitive_resources::is_sensitive("customer"));
        assert!(sensitive_resources::is_sensitive("dye_recipe"));
        assert!(sensitive_resources::is_sensitive("finance_report"));
        assert!(!sensitive_resources::is_sensitive("product"));
        assert!(!sensitive_resources::is_sensitive("inventory"));
    }
}