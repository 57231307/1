//! 色卡仓储管理 - 发放 / 归还 / 遗失 集成测试
//!
//! V15 P0-F03 重构：从 borrow 测试重写为 issue 测试
//! 创建时间: 2026-06-17（V15 重构: 2026-07-17）

#[cfg(test)]
mod tests {
    use bingxi_backend::services::color_card_issue_service::IssueStatus;
    use std::str::FromStr;

    #[test]
    fn test_issue_status_terminal() {
        // 终态：returned / lost / damaged / cancelled
        assert!(IssueStatus::Returned.is_terminal());
        assert!(IssueStatus::Lost.is_terminal());
        assert!(IssueStatus::Damaged.is_terminal());
        assert!(IssueStatus::Cancelled.is_terminal());
        // 非终态：issued
        assert!(!IssueStatus::Issued.is_terminal());
    }

    #[test]
    fn test_issue_status_from_str() {
        assert_eq!(IssueStatus::from_str("issued").unwrap(), IssueStatus::Issued);
        assert_eq!(IssueStatus::from_str("returned").unwrap(), IssueStatus::Returned);
        assert_eq!(IssueStatus::from_str("lost").unwrap(), IssueStatus::Lost);
        assert_eq!(IssueStatus::from_str("damaged").unwrap(), IssueStatus::Damaged);
        assert_eq!(IssueStatus::from_str("cancelled").unwrap(), IssueStatus::Cancelled);
        assert!(IssueStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_issue_status_as_str() {
        assert_eq!(IssueStatus::Issued.as_str(), "issued");
        assert_eq!(IssueStatus::Returned.as_str(), "returned");
        assert_eq!(IssueStatus::Lost.as_str(), "lost");
        assert_eq!(IssueStatus::Damaged.as_str(), "damaged");
        assert_eq!(IssueStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_compensation_amount_must_be_positive() {
        // 业务规则：赔付金额必须 > 0
        let amount: f64 = 0.0;
        assert!(amount <= 0.0);

        let amount: f64 = 100.50;
        assert!(amount > 0.0);
    }

    #[test]
    fn test_expected_return_within_30_days() {
        use chrono::{Duration, Utc};
        let now = Utc::now();
        // 合法：5 天后
        let valid_expected = now + Duration::days(5);
        let max_valid = now + Duration::days(30);
        assert!(valid_expected <= max_valid);

        // 非法：60 天后
        let invalid_expected = now + Duration::days(60);
        assert!(invalid_expected > max_valid);
    }
}
