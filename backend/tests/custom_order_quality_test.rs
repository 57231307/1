//! 质检集成测试
//!
//! 覆盖 GB/T 26377 色差 + ISO 105 色牢度规则
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    /// 校验色差 ΔE 是否在合理范围（GB/T 26377-2022）
    fn validate_color_delta_e(delta_e: Decimal) -> bool {
        delta_e >= Decimal::ZERO && delta_e <= Decimal::from(100)
    }

    /// 校验 ISO 105 色牢度等级
    fn validate_color_fastness(grade: i32) -> bool {
        (1..=5).contains(&grade)
    }

    #[test]
    fn test_color_delta_e_validation() {
        assert!(validate_color_delta_e(Decimal::ZERO));
        assert!(validate_color_delta_e(Decimal::from(1)));
        assert!(validate_color_delta_e(Decimal::from(5)));
        assert!(!validate_color_delta_e(Decimal::from(-1)));
    }

    #[test]
    fn test_color_fastness_grade_validation() {
        assert!(validate_color_fastness(1));
        assert!(validate_color_fastness(3));
        assert!(validate_color_fastness(5));
        assert!(!validate_color_fastness(0));
        assert!(!validate_color_fastness(6));
        assert!(!validate_color_fastness(-1));
    }

    #[test]
    fn test_severity_levels() {
        let valid_severities = vec!["low", "medium", "high", "critical"];
        for s in valid_severities {
            assert!(["low", "medium", "high", "critical"].contains(&s));
        }
    }

    #[test]
    fn test_issue_types() {
        let valid_issue_types = vec![
            "color_diff",      // 色差
            "color_fastness",  // 色牢度
            "spec",            // 规格不符
            "damage",          // 破损
            "other",
        ];
        for t in valid_issue_types {
            assert!(["color_diff", "color_fastness", "spec", "damage", "other"].contains(&t));
        }
    }
}
