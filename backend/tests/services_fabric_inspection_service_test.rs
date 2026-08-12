#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use super::*;

    // ===== 四分制扣分计算测试 =====

    #[test]
    fn test_calculate_four_point_points_normal() {
        // ≤3寸 = 1分
        assert_eq!(
            calculate_four_point_points(Decimal::new(3, 0), false, false),
            1
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(0, 0), false, false),
            1
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(2, 0), false, false),
            1
        );

        // 3-6寸 = 2分
        assert_eq!(
            calculate_four_point_points(Decimal::new(4, 0), false, false),
            2
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(6, 0), false, false),
            2
        );

        // 6-9寸 = 3分
        assert_eq!(
            calculate_four_point_points(Decimal::new(7, 0), false, false),
            3
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(9, 0), false, false),
            3
        );

        // >9寸 = 4分
        assert_eq!(
            calculate_four_point_points(Decimal::new(10, 0), false, false),
            4
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(36, 0), false, false),
            4
        );
    }

    #[test]
    fn test_calculate_four_point_points_hole_and_continuous() {
        // 破洞不论大小一律4分
        assert_eq!(
            calculate_four_point_points(Decimal::new(0, 0), true, false),
            4
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(1, 0), true, false),
            4
        );

        // 连续性疵点不论大小一律4分
        assert_eq!(
            calculate_four_point_points(Decimal::new(0, 0), false, true),
            4
        );
        assert_eq!(
            calculate_four_point_points(Decimal::new(1, 0), false, true),
            4
        );
    }

    // ===== 十分制扣分计算测试 =====

    #[test]
    fn test_calculate_ten_point_points_warp() {
        // 破洞 = 10分
        assert_eq!(
            calculate_ten_point_points(Decimal::new(1, 0), "warp", true, false),
            10
        );

        // 经向：1寸下=1
        assert_eq!(
            calculate_ten_point_points(Decimal::new(0, 0), "warp", false, false),
            1
        );

        // 经向：1-5寸=3
        assert_eq!(
            calculate_ten_point_points(Decimal::new(1, 0), "warp", false, false),
            3
        );
        assert_eq!(
            calculate_ten_point_points(Decimal::new(5, 0), "warp", false, false),
            3
        );

        // 经向：5-10寸=5
        assert_eq!(
            calculate_ten_point_points(Decimal::new(6, 0), "warp", false, false),
            5
        );
        assert_eq!(
            calculate_ten_point_points(Decimal::new(10, 0), "warp", false, false),
            5
        );

        // 经向：10-36寸=10
        assert_eq!(
            calculate_ten_point_points(Decimal::new(11, 0), "warp", false, false),
            10
        );
        assert_eq!(
            calculate_ten_point_points(Decimal::new(36, 0), "warp", false, false),
            10
        );
    }

    #[test]
    fn test_calculate_ten_point_points_weft() {
        // 纬向：1寸下=1
        assert_eq!(
            calculate_ten_point_points(Decimal::new(0, 0), "weft", false, false),
            1
        );

        // 纬向：1-5寸=3
        assert_eq!(
            calculate_ten_point_points(Decimal::new(3, 0), "weft", false, false),
            3
        );

        // 纬向：5寸-半门幅=5
        assert_eq!(
            calculate_ten_point_points(Decimal::new(6, 0), "weft", false, false),
            5
        );

        // 纬向：半门幅以上=10
        assert_eq!(
            calculate_ten_point_points(Decimal::new(6, 0), "weft", false, true),
            10
        );
    }

    // ===== 每百平方码分数计算测试 =====

    #[test]
    fn test_calculate_points_per_100_sq_yards() {
        // (655 × 36 × 100) / (2500 × 55) = 17.1
        let result =
            calculate_points_per_100_sq_yards(655, Decimal::new(2500, 0), Decimal::new(55, 0))
                .unwrap();
        let expected = Decimal::new(171, 1); // 17.1
        assert_eq!(result.round_dp(1), expected);
    }

    #[test]
    fn test_calculate_points_per_100_sq_yards_invalid_input() {
        // 受检码数为0
        let result = calculate_points_per_100_sq_yards(10, Decimal::ZERO, Decimal::new(55, 0));
        assert!(result.is_err());

        // 幅宽为0
        let result = calculate_points_per_100_sq_yards(10, Decimal::new(100, 0), Decimal::ZERO);
        assert!(result.is_err());
    }

    // ===== 等级判定测试 =====

    #[test]
    fn test_determine_grade_by_four_point() {
        // ≤40 = 首级
        assert_eq!(
            determine_grade_by_four_point(Decimal::new(40, 0)),
            fabric_grade::FIRST
        );
        assert_eq!(
            determine_grade_by_four_point(Decimal::new(0, 0)),
            fabric_grade::FIRST
        );
        assert_eq!(
            determine_grade_by_four_point(Decimal::new(16, 0)),
            fabric_grade::FIRST
        );

        // >40 = 次级
        assert_eq!(
            determine_grade_by_four_point(Decimal::new(41, 0)),
            fabric_grade::SECOND
        );
        assert_eq!(
            determine_grade_by_four_point(Decimal::new(100, 0)),
            fabric_grade::SECOND
        );
    }

    #[test]
    fn test_determine_grade_by_ten_point() {
        // 总扣分 < 总码数 = 首级
        assert_eq!(
            determine_grade_by_ten_point(50, Decimal::new(100, 0)),
            fabric_grade::FIRST
        );
        assert_eq!(
            determine_grade_by_ten_point(0, Decimal::new(100, 0)),
            fabric_grade::FIRST
        );

        // 总扣分 ≥ 总码数 = 次级
        assert_eq!(
            determine_grade_by_ten_point(100, Decimal::new(100, 0)),
            fabric_grade::SECOND
        );
        assert_eq!(
            determine_grade_by_ten_point(150, Decimal::new(100, 0)),
            fabric_grade::SECOND
        );
    }

    // ===== 疵点类型校验测试 =====

    #[test]
    fn test_validate_defect_type_valid() {
        assert!(FabricDefectService::validate_defect_type("broken_end").is_ok());
        assert!(FabricDefectService::validate_defect_type("hole").is_ok());
        assert!(FabricDefectService::validate_defect_type("other").is_ok());
    }

    #[test]
    fn test_validate_defect_type_invalid() {
        assert!(FabricDefectService::validate_defect_type("invalid_type").is_err());
        assert!(FabricDefectService::validate_defect_type("").is_err());
    }
}