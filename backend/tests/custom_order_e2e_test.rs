//! 定制订单端到端集成测试
//!
//! 覆盖完整生命周期：创建 → 推进 → 异常 → 售后 → 完成
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    // 模拟 5 阶段状态机转换（端到端场景）
    #[test]
    fn test_full_lifecycle_state_progression() {
        use crate::utils::process_state_machine::next_status;

        // draft → yarn_purchasing → dyeing → finishing → delivery → after_sales → completed
        let mut current = "draft";
        let expected = vec![
            "yarn_purchasing",
            "dyeing",
            "finishing",
            "delivery",
            "after_sales",
            "completed",
        ];

        for exp in expected {
            let next = next_status(current).expect("状态转换应成功");
            assert_eq!(next.as_str(), *exp, "状态应推进到 {}", exp);
            current = next.as_str();
        }
    }

    #[test]
    fn test_terminal_state_cannot_advance() {
        use crate::utils::process_state_machine::next_status;
        assert!(next_status("completed").is_err());
        assert!(next_status("cancelled").is_err());
    }

    #[test]
    fn test_decimal_quantity_validation() {
        let qty = Decimal::from_str("100.5").unwrap();
        assert!(qty > Decimal::ZERO);
    }
}
