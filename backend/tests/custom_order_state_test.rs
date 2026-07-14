//! 状态机集成测试
//!
//! 覆盖 5 阶段状态转换矩阵
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use bingxi_backend::utils::process_state_machine::{can_transition, next_status, CustomOrderStatus};

    #[test]
    fn test_state_machine_complete_matrix() {
        // 验证所有 6 个合法转换
        let valid_transitions = vec![
            ("draft", "yarn_purchasing"),
            ("yarn_purchasing", "dyeing"),
            ("dyeing", "finishing"),
            ("finishing", "delivery"),
            ("delivery", "after_sales"),
            ("after_sales", "completed"),
        ];

        for (from, to) in valid_transitions {
            assert!(can_transition(from, to), "合法转换: {} → {}", from, to);
        }
    }

    #[test]
    fn test_cancellation_paths() {
        // 任意非终态都可取消
        let non_terminal = vec!["draft", "yarn_purchasing", "dyeing", "finishing", "delivery", "after_sales"];
        for from in non_terminal {
            assert!(can_transition(from, "cancelled"), "{} 应可取消", from);
        }

        // 终态不可取消
        assert!(!can_transition("completed", "cancelled"));
        assert!(!can_transition("cancelled", "cancelled"));
    }

    #[test]
    fn test_invalid_skips() {
        // 阶段跳跃应被拒绝
        assert!(!can_transition("draft", "dyeing"));
        assert!(!can_transition("draft", "delivery"));
        assert!(!can_transition("yarn_purchasing", "finishing"));
    }

    #[test]
    fn test_status_serialization() {
        assert_eq!(CustomOrderStatus::Draft.as_str(), "draft");
        assert_eq!(CustomOrderStatus::YarnPurchasing.as_str(), "yarn_purchasing");
        assert_eq!(CustomOrderStatus::Dyeing.as_str(), "dyeing");
        assert_eq!(CustomOrderStatus::Finishing.as_str(), "finishing");
        assert_eq!(CustomOrderStatus::Delivery.as_str(), "delivery");
        assert_eq!(CustomOrderStatus::AfterSales.as_str(), "after_sales");
        assert_eq!(CustomOrderStatus::Completed.as_str(), "completed");
        assert_eq!(CustomOrderStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_status_parsing() {
        // 批次 415：CustomOrderStatus 实现了 FromStr trait 返回 Result，
        // 使用 .parse().ok() 转为 Option 保持原有测试语义
        assert_eq!("draft".parse::<CustomOrderStatus>().ok(), Some(CustomOrderStatus::Draft));
        assert_eq!("yarn_purchasing".parse::<CustomOrderStatus>().ok(), Some(CustomOrderStatus::YarnPurchasing));
        assert_eq!("invalid".parse::<CustomOrderStatus>().ok(), None);
    }

    #[test]
    fn test_next_status_returns_correct_value() {
        let next = next_status("draft").unwrap();
        assert_eq!(next, CustomOrderStatus::YarnPurchasing);
    }
}
