use bingxi_backend::models::status::quality_dyeing::*;
use bingxi_backend::services::fabric_inspection_service::calculate_four_point_points;
use bingxi_backend::services::fabric_inspection_service::calculate_points_per_100_sq_yards;
use bingxi_backend::services::fabric_inspection_service::calculate_ten_point_points;
use bingxi_backend::services::fabric_inspection_service::determine_grade_by_four_point;
use bingxi_backend::services::fabric_inspection_service::determine_grade_by_ten_point;
use bingxi_backend::services::fabric_inspection_service::sum_four_point_points_capped_per_yard;
use rust_decimal::Decimal;

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
        calculate_points_per_100_sq_yards(655, Decimal::new(2500, 0), Decimal::new(55, 0)).unwrap();
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
use bingxi_backend::models::status::fabric_grade;
use bingxi_backend::services::fabric_inspection_service::FabricDefectService;

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

// ===== 四分制同码封顶聚合测试 =====

#[test]
fn test_per_yard_cap_multiple_defects_in_one_yard() {
    // 同一码位（读数 1.20 1.50 1.99 都归第 1 码）三枚 4 分疵点，线性合计 12，封顶后计 4
    let defects = [
        (Decimal::new(120, 2), 4),
        (Decimal::new(150, 2), 4),
        (Decimal::new(199, 2), 4),
    ];
    assert_eq!(sum_four_point_points_capped_per_yard(&defects), 4);
}

#[test]
fn test_per_yard_cap_bucket_boundary_is_floor() {
    // 分码判据是 position_yards 向下取整，0.99 归第 0 码，1.00 归第 1 码，
    // 两码各 4 分分别封顶，合计 8（若误把 0.99 与 1.00 归同码会得到 4）
    let defects = [(Decimal::new(99, 2), 4), (Decimal::new(100, 2), 4)];
    assert_eq!(sum_four_point_points_capped_per_yard(&defects), 8);
}

#[test]
fn test_per_yard_cap_bucket_sum_below_cap_kept() {
    // 同一码位扣分合计 2+1 未超 4 时按实际计入，排除每码无条件记满 4 的错误语义
    let defects = [(Decimal::new(500, 2), 2), (Decimal::new(520, 2), 1)];
    assert_eq!(sum_four_point_points_capped_per_yard(&defects), 3);
}

#[test]
fn test_per_yard_cap_mixed_buckets() {
    // 第 5 码桶内 2+2+3 合计 7 封顶计 4，第 6 码 1 分照计，总 5
    // （线性合计 8，每码只取最大扣分误法得 4，每码无条件记满误法得 8，断言 5 同时钉死错误语义）
    let defects = [
        (Decimal::new(520, 2), 2),
        (Decimal::new(580, 2), 2),
        (Decimal::new(590, 2), 3),
        (Decimal::new(620, 2), 1),
    ];
    assert_eq!(sum_four_point_points_capped_per_yard(&defects), 5);
}

#[test]
fn test_per_yard_cap_scattered_across_yards_unaffected() {
    // 10 枚 4 分疵点散布第 3 码至第 30 码各一枚，封顶不改变总数 40
    let defects: Vec<(Decimal, i32)> = (3..=30)
        .step_by(3)
        .map(|yard| (Decimal::new(yard, 0), 4))
        .collect();
    assert_eq!(defects.len(), 10);
    assert_eq!(sum_four_point_points_capped_per_yard(&defects), 40);
}

#[test]
fn test_per_yard_cap_empty_defects() {
    let defects: Vec<(Decimal, i32)> = Vec::new();
    assert_eq!(sum_four_point_points_capped_per_yard(&defects), 0);
}
