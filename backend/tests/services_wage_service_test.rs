#[cfg(test)]
mod tests {
    use bingxi_backend::models::status::wage_rate_status;
    use bingxi_backend::services::quality_inspection_service::{
        QUALITY_GRADE_A, QUALITY_GRADE_B, QUALITY_GRADE_C,
    };
    use bingxi_backend::services::wage_service::{
        compute_qualification_rate, determine_grade_by_qualification_rate,
    };
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::Decimal;

    // ===== compute_qualification_rate 合格率计算 =====

    /// test_hgljs_zcqk（验证 actual=100, qualified=95 时合格率为 95%。）
    #[test]
    fn test_hgljs_zcqk() {
        let rate =
            compute_qualification_rate(Some(Decimal::new(100, 0)), Some(Decimal::new(95, 0)));
        assert_eq!(rate, Decimal::new(95, 0));
    }

    /// test_hgljs_qhg（验证 actual=100, qualified=100 时合格率为 100%。）
    #[test]
    fn test_hgljs_qhg() {
        let rate =
            compute_qualification_rate(Some(Decimal::new(100, 0)), Some(Decimal::new(100, 0)));
        assert_eq!(rate, Decimal::new(100, 0));
    }

    /// test_hgljs_lcl（验证 actual=0 时合格率为 0（避免除零错误）。）
    #[test]
    fn test_hgljs_lcl() {
        let rate = compute_qualification_rate(Some(Decimal::ZERO), Some(Decimal::ZERO));
        assert_eq!(rate, Decimal::ZERO);
    }

    /// test_hgljs_nonealcl（验证 None 时按 0 处理。）
    #[test]
    fn test_hgljs_nonealcl() {
        let rate = compute_qualification_rate(None, None);
        assert_eq!(rate, Decimal::ZERO);
    }

    // ===== determine_grade_by_qualification_rate 等级判定 =====

    /// test_djpd_aj_95ys（验证合格率 ≥ 95% 判定为 A 级。）
    #[test]
    fn test_djpd_aj_95ys() {
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(95, 0)),
            QUALITY_GRADE_A
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(100, 0)),
            QUALITY_GRADE_A
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(995, 1)), // 99.5
            QUALITY_GRADE_A
        );
    }

    /// test_djpd_bj_80d95qj（验证合格率 80-95% 判定为 B 级。）
    #[test]
    fn test_djpd_bj_80d95qj() {
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(80, 0)),
            QUALITY_GRADE_B
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(85, 0)),
            QUALITY_GRADE_B
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(9499, 2)), // 94.99
            QUALITY_GRADE_B
        );
    }

    /// test_djpd_cj_80yx（验证合格率 < 80% 判定为 C 级。）
    #[test]
    fn test_djpd_cj_80yx() {
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(79, 0)),
            QUALITY_GRADE_C
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::new(50, 0)),
            QUALITY_GRADE_C
        );
        assert_eq!(
            determine_grade_by_qualification_rate(Decimal::ZERO),
            QUALITY_GRADE_C
        );
    }

    // ===== determine_grade_ratio 等级系数获取 =====

    /// test_djxshq_gjb（验证 A/B/C 级返回对应的工价等级系数。）
    #[test]
    fn test_djxshq_gjb() {
        // 构造一个 Mock 工价模型
        let rate = RateModel {
            id: 1,
            rate_no: "PWR-TEST-001".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0), // 5 元/kg
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1), // 1.0
            grade_b_ratio: Decimal::new(8, 1),  // 0.8
            grade_c_ratio: Decimal::ZERO,
            effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            effective_to: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        assert_eq!(
            determine_grade_ratio(QUALITY_GRADE_A, &rate),
            Decimal::new(10, 1)
        );
        assert_eq!(
            determine_grade_ratio(QUALITY_GRADE_B, &rate),
            Decimal::new(8, 1)
        );
        assert_eq!(determine_grade_ratio(QUALITY_GRADE_C, &rate), Decimal::ZERO);
        // 未知等级返回 0
        assert_eq!(determine_grade_ratio("X", &rate), Decimal::ZERO);
    }

    // ===== calculate_wage_for_step 工资计算 =====

    /// test_gzjs_jj_ajqe（验证计件工价 + A 级（100%合格率）= 合格产量 × 计件单价 × 1.0。）
    #[test]
    fn test_gzjs_jj_ajqe() {
        let rate = RateModel {
            id: 1,
            rate_no: "PWR-TEST-002".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0), // 5 元/kg
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            effective_to: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // actual=100kg, qualified=100kg, 100% 合格率 → A 级
        let (grade, ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(100, 0)),
            Some(120),
        );

        assert_eq!(grade, QUALITY_GRADE_A);
        assert_eq!(ratio, Decimal::new(10, 1));
        assert_eq!(piece_wage, Decimal::new(500, 0)); // 100 × 5 × 1.0 = 500
        assert_eq!(time_wage, Decimal::ZERO); // 计件类型，计时为 0
        assert_eq!(total, Decimal::new(500, 0));
    }

    /// test_gzjs_jj_bj8z（验证计件工价 + B 级（85%合格率）= 合格产量 × 计件单价 × 0.8。）
    #[test]
    fn test_gzjs_jj_bj8z() {
        let rate = RateModel {
            id: 2,
            rate_no: "PWR-TEST-003".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0),
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            effective_to: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // actual=100kg, qualified=85kg, 85% 合格率 → B 级
        let (grade, ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(85, 0)),
            Some(120),
        );

        assert_eq!(grade, QUALITY_GRADE_B);
        assert_eq!(ratio, Decimal::new(8, 1));
        // 85 × 5 × 0.8 = 340
        assert_eq!(piece_wage, Decimal::new(340, 0));
        assert_eq!(time_wage, Decimal::ZERO);
        assert_eq!(total, Decimal::new(340, 0));
    }

    /// test_gzjs_jj_cjbj（验证计件工价 + C 级（50%合格率）= 工资为 0。）
    #[test]
    fn test_gzjs_jj_cjbj() {
        let rate = RateModel {
            id: 3,
            rate_no: "PWR-TEST-004".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::PIECE.to_string(),
            piece_price: Decimal::new(5, 0),
            time_price: Decimal::ZERO,
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            effective_to: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // actual=100kg, qualified=50kg, 50% 合格率 → C 级
        let (grade, ratio, piece_wage, _time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(50, 0)),
            Some(120),
        );

        assert_eq!(grade, QUALITY_GRADE_C);
        assert_eq!(ratio, Decimal::ZERO);
        assert_eq!(piece_wage, Decimal::ZERO);
        assert_eq!(total, Decimal::ZERO);
    }

    /// test_gzjs_js_ags（验证计时工价 = 工时 × 计时单价 × 等级系数。）
    #[test]
    fn test_gzjs_js_ags() {
        let rate = RateModel {
            id: 4,
            rate_no: "PWR-TEST-005".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::TIME.to_string(),
            piece_price: Decimal::ZERO,
            time_price: Decimal::new(2, 0), // 2 元/分钟
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            effective_to: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // 100% 合格率 → A 级，120 分钟
        let (_grade, _ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(100, 0)),
            Some(120),
        );

        // 120 × 2 × 1.0 = 240
        assert_eq!(piece_wage, Decimal::ZERO); // 计时类型，计件为 0
        assert_eq!(time_wage, Decimal::new(240, 0));
        assert_eq!(total, Decimal::new(240, 0));
    }

    /// test_gzjs_hh_jjjjs（验证混合工价 = 计件 + 计时。）
    #[test]
    fn test_gzjs_hh_jjjjs() {
        let rate = RateModel {
            id: 5,
            rate_no: "PWR-TEST-006".to_string(),
            process_route_id: 1,
            route_code: "DYE".to_string(),
            route_name: "染色".to_string(),
            wage_type: wage_type::MIXED.to_string(),
            piece_price: Decimal::new(5, 0),
            time_price: Decimal::new(2, 0),
            grade_a_ratio: Decimal::new(10, 1),
            grade_b_ratio: Decimal::new(8, 1),
            grade_c_ratio: Decimal::ZERO,
            effective_from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            effective_to: None,
            workshop: None,
            status: wage_rate_status::ACTIVE.to_string(),
            remarks: None,
            is_deleted: false,
            created_by: None,
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
        };

        // 85% 合格率 → B 级，100kg 合格产量，120 分钟
        let (_grade, _ratio, piece_wage, time_wage, total) = calculate_wage_for_step(
            &rate,
            Some(Decimal::new(100, 0)),
            Some(Decimal::new(85, 0)),
            Some(120),
        );

        // piece_wage = 85 × 5 × 0.8 = 340
        // time_wage = 120 × 2 × 0.8 = 192
        assert_eq!(piece_wage, Decimal::new(340, 0));
        assert_eq!(time_wage, Decimal::new(192, 0));
        assert_eq!(total, Decimal::new(532, 0)); // 340 + 192
    }

    // ===== parse_worker_ids 工人IDs解析 =====

    /// test_gridsjx_zcqk
    #[test]
    fn test_gridsjx_zcqk() {
        let ids = parse_worker_ids(&Some("1,2,3".to_string()));
        assert_eq!(ids, vec![1, 2, 3]);
    }

    /// test_gridsjx_dkg
    #[test]
    fn test_gridsjx_dkg() {
        let ids = parse_worker_ids(&Some("1, 2, 3".to_string()));
        assert_eq!(ids, vec![1, 2, 3]);
    }

    /// test_gridsjx_kz
    #[test]
    fn test_gridsjx_kz() {
        assert!(parse_worker_ids(&None).is_empty());
        assert!(parse_worker_ids(&Some(String::new())).is_empty());
        assert!(parse_worker_ids(&Some("  ".to_string())).is_empty());
    }

    /// test_gridsjx_ffzgl
    #[test]
    fn test_gridsjx_ffzgl() {
        let ids = parse_worker_ids(&Some("1,abc,3,".to_string()));
        assert_eq!(ids, vec![1, 3]);
    }

    // ===== split_wage_among_workers 工资按人均分配 =====

    /// test_gzarjfp_dr
    #[test]
    fn test_gzarjfp_dr() {
        let wage = Decimal::new(500, 0);
        assert_eq!(split_wage_among_workers(wage, 1), Decimal::new(500, 0));
    }

    /// test_gzarjfp_drzc
    #[test]
    fn test_gzarjfp_drzc() {
        let wage = Decimal::new(500, 0);
        assert_eq!(split_wage_among_workers(wage, 5), Decimal::new(100, 0));
    }

    /// test_gzarjfp_lr
    #[test]
    fn test_gzarjfp_lr() {
        let wage = Decimal::new(500, 0);
        assert_eq!(split_wage_among_workers(wage, 0), Decimal::ZERO);
    }

    /// test_gzarjfp_fzcqxs
    #[test]
    fn test_gzarjfp_fzcqxs() {
        let wage = Decimal::new(100, 0);
        // 100 / 3 = 33.33...
        let result = split_wage_among_workers(wage, 3);
        let f = result.to_f64().unwrap();
        assert!((f - 33.3333).abs() < 0.01);
    }
}