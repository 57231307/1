#[cfg(test)]
mod tests {
    use bingxi_backend::utils::unwrap_safe::decs;
    use bingxi_backend::services::quality_inspection_service::{
        determine_quality_grade, grade_a_threshold, grade_b_threshold,
        validate_handling_method_by_grade, HANDLING_DOWNGRADE_SALE, HANDLING_REWORK,
        HANDLING_SCRAP, QUALITY_GRADE_A, QUALITY_GRADE_B, QUALITY_GRADE_C,
    };

    // ===== determine_quality_grade 合格率分级判定 =====

    /// test_zjfj_aj_hgldb（验证 qualification_rate >= 95% 判定为 A 级（合格），覆盖边界值 95%。）
    #[test]
    fn test_zjfj_aj_hgldb() {
        // 边界：恰好 95% → A 级
        assert_eq!(determine_quality_grade(Some(decs!("95"))), QUALITY_GRADE_A);
        // 高于 95% → A 级
        assert_eq!(determine_quality_grade(Some(decs!("100"))), QUALITY_GRADE_A);
        assert_eq!(
            determine_quality_grade(Some(decs!("99.5"))),
            QUALITY_GRADE_A
        );
    }

    /// test_zjfj_bj_rbjsqj（验证 80% <= rate < 95% 判定为 B 级（让步接收，降级销售）。）
    #[test]
    fn test_zjfj_bj_rbjsqj() {
        // 边界：恰好 80% → B 级
        assert_eq!(determine_quality_grade(Some(decs!("80"))), QUALITY_GRADE_B);
        // 区间内 → B 级
        assert_eq!(determine_quality_grade(Some(decs!("85"))), QUALITY_GRADE_B);
        assert_eq!(
            determine_quality_grade(Some(decs!("94.99"))),
            QUALITY_GRADE_B
        );
    }

    /// test_zjfj_cj_bhgqj（验证 rate < 80% 判定为 C 级（不合格，返工或报废）。）
    #[test]
    fn test_zjfj_cj_bhgqj() {
        // 边界：恰好低于 80% → C 级
        assert_eq!(
            determine_quality_grade(Some(decs!("79.99"))),
            QUALITY_GRADE_C
        );
        assert_eq!(determine_quality_grade(Some(decs!("50"))), QUALITY_GRADE_C);
        assert_eq!(
            determine_quality_grade(Some(Decimal::ZERO)),
            QUALITY_GRADE_C
        );
    }

    /// test_zjfj_noneswlhgl（验证 qualification_rate 为 None 时按 0% 处理为 C 级。）
    #[test]
    fn test_zjfj_noneswlhgl() {
        assert_eq!(determine_quality_grade(None), QUALITY_GRADE_C);
    }

    // ===== validate_handling_method_by_grade 等级与处理方式匹配校验 =====

    /// test_djclfsjy_ajpwxbhgcl（A 级（合格）品调用任何处理方式都应返回错误。）
    #[test]
    fn test_djclfsjy_ajpwxbhgcl() {
        // A 级 + 任意处理方式 → 拒绝
        assert!(
            validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_DOWNGRADE_SALE).is_err()
        );
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_REWORK).is_err());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_SCRAP).is_err());

        let err = validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_REWORK).unwrap_err();
        assert!(err.to_string().contains("A 级"));
    }

    /// test_djclfsjy_bjpbxjjxs（B 级（让步接收）品处理方式必须为 downgrade_sale，其他拒绝。）
    #[test]
    fn test_djclfsjy_bjpbxjjxs() {
        // B 级 + 降级销售 → 放行
        assert!(
            validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_DOWNGRADE_SALE).is_ok()
        );
        // B 级 + 返工/报废 → 拒绝
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_REWORK).is_err());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_SCRAP).is_err());

        let err = validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_REWORK).unwrap_err();
        assert!(err.to_string().contains("B 级"));
        assert!(err.to_string().contains("降级销售"));
    }

    /// test_djclfsjy_cjpfghbf（C 级（不合格）品处理方式必须为 rework 或 scrap，其他拒绝。）
    #[test]
    fn test_djclfsjy_cjpfghbf() {
        // C 级 + 返工/报废 → 放行
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_REWORK).is_ok());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_SCRAP).is_ok());
        // C 级 + 降级销售 → 拒绝
        assert!(
            validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_DOWNGRADE_SALE).is_err()
        );

        let err = validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_DOWNGRADE_SALE)
            .unwrap_err();
        assert!(err.to_string().contains("C 级"));
        assert!(err.to_string().contains("返工"));
        assert!(err.to_string().contains("报废"));
    }

    /// test_djclfsjy_wzdjjj（grade 为 D/X/空字符串等非 A/B/C 值时返回错误。）
    #[test]
    fn test_djclfsjy_wzdjjj() {
        assert!(validate_handling_method_by_grade("D", HANDLING_REWORK).is_err());
        assert!(validate_handling_method_by_grade("X", HANDLING_SCRAP).is_err());
        assert!(validate_handling_method_by_grade("", HANDLING_DOWNGRADE_SALE).is_err());

        let err = validate_handling_method_by_grade("D", HANDLING_REWORK).unwrap_err();
        assert!(err.to_string().contains("未知质检等级"));
    }

    /// test_djclzzqx（校验 A/B/C 等级常量与处理方式常量值，避免硬编码字符串拼写错误。）
    #[test]
    fn test_djclzzqx() {
        assert_eq!(QUALITY_GRADE_A, "A");
        assert_eq!(QUALITY_GRADE_B, "B");
        assert_eq!(QUALITY_GRADE_C, "C");
        assert_eq!(grade_a_threshold(), decs!("95"));
        assert_eq!(grade_b_threshold(), decs!("80"));
        assert_eq!(HANDLING_DOWNGRADE_SALE, "downgrade_sale");
        assert_eq!(HANDLING_REWORK, "rework");
        assert_eq!(HANDLING_SCRAP, "scrap");
    }

    /// test_decimalyzjxzqx（校验 grade_a_threshold / grade_b_threshold 通过 Decimal::new 构造的值正确。）
    #[test]
    fn test_decimalyzjxzqx() {
        let d = Decimal::from_str("95").unwrap();
        assert_eq!(grade_a_threshold(), d);
        let d = Decimal::from_str("80").unwrap();
        assert_eq!(grade_b_threshold(), d);
    }
}