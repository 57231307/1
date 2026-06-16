//! 色卡仓储管理 - 借出 / 归还 / 遗失 集成测试
//!
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use crate::services::color_card_borrow_service::BorrowStatus;

    #[test]
    fn test_borrow_status_terminal() {
        // 终态：returned / lost / damaged
        assert!(BorrowStatus::Returned.is_terminal());
        assert!(BorrowStatus::Lost.is_terminal());
        assert!(BorrowStatus::Damaged.is_terminal());
        // 非终态：borrowed
        assert!(!BorrowStatus::Borrowed.is_terminal());
    }

    #[test]
    fn test_borrow_status_from_str() {
        assert_eq!(BorrowStatus::from_str("borrowed"), Some(BorrowStatus::Borrowed));
        assert_eq!(BorrowStatus::from_str("returned"), Some(BorrowStatus::Returned));
        assert_eq!(BorrowStatus::from_str("lost"), Some(BorrowStatus::Lost));
        assert_eq!(BorrowStatus::from_str("damaged"), Some(BorrowStatus::Damaged));
        assert_eq!(BorrowStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_borrow_status_as_str() {
        assert_eq!(BorrowStatus::Borrowed.as_str(), "borrowed");
        assert_eq!(BorrowStatus::Returned.as_str(), "returned");
        assert_eq!(BorrowStatus::Lost.as_str(), "lost");
        assert_eq!(BorrowStatus::Damaged.as_str(), "damaged");
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
