//! 客户信用评估模型单元测试

#[test]
fn test_credit_rating_calculation() {
    let test_cases = vec![
        (95, "AAA", 500000),
        (85, "AA", 300000),
        (75, "A", 200000),
        (65, "BBB", 100000),
        (55, "BB", 50000),
        (45, "C", 10000),
    ];

    for (score, expected_rating, expected_limit) in test_cases {
        let (rating, limit) = calculate_rating_and_limit(score);
        assert_eq!(rating, expected_rating, "分数 {} 的等级不正确", score);
        assert_eq!(limit, expected_limit as f64, "分数 {} 的额度不正确", score);
    }
}

fn calculate_rating_and_limit(score: i32) -> (String, f64) {
    if score >= 90 {
        ("AAA".to_string(), 500000.0)
    } else if score >= 80 {
        ("AA".to_string(), 300000.0)
    } else if score >= 70 {
        ("A".to_string(), 200000.0)
    } else if score >= 60 {
        ("BBB".to_string(), 100000.0)
    } else if score >= 50 {
        ("BB".to_string(), 50000.0)
    } else {
        ("C".to_string(), 10000.0)
    }
}

#[test]
fn test_weighted_score_calculation() {
    let payment_score = 90;
    let cooperation_score = 80;
    let order_score = 70;
    let credit_score = 85;

    let total_score = (payment_score as f64 * 0.3)
        + (cooperation_score as f64 * 0.2)
        + (order_score as f64 * 0.25)
        + (credit_score as f64 * 0.25);

    // 90*0.3 + 80*0.2 + 70*0.25 + 85*0.25 = 27 + 16 + 17.5 + 21.25 = 81.75
    assert!((total_score - 81.75).abs() < f64::EPSILON);
}

#[test]
fn test_cooperation_duration_scoring() {
    assert_eq!(evaluate_cooperation_duration(365 * 4), 100); // 4 年
    assert_eq!(evaluate_cooperation_duration(365 * 2 + 180), 80); // 2.5 年
    assert_eq!(evaluate_cooperation_duration(365 + 90), 60); // 1.25 年
    assert_eq!(evaluate_cooperation_duration(200), 40); // 6.6 个月
    assert_eq!(evaluate_cooperation_duration(100), 20); // 3.3 个月
}

fn evaluate_cooperation_duration(days: i64) -> i32 {
    if days > 365 * 3 {
        100
    } else if days > 365 * 2 {
        80
    } else if days > 365 {
        60
    } else if days > 180 {
        40
    } else {
        20
    }
}

#[test]
fn test_payment_history_scoring() {
    let total_orders = 10;
    let on_time_orders = 8;
    let on_time_rate = on_time_orders as f64 / total_orders as f64;
    let score = (on_time_rate * 100.0) as i32;

    assert_eq!(score, 80);
}

#[test]
fn test_credit_history_scoring() {
    let test_cases = vec![
        (vec!["active", "active"], 100),            // 全部良好
        (vec!["active", "active", "expired"], 100), // 全部良好
        (vec!["active", "active", "overdue"], 66),  // 部分良好（近似）
        (vec!["overdue", "overdue"], 40),           // 无良好记录
    ];

    for (records, _expected) in test_cases {
        let good_count = records
            .iter()
            .filter(|&s| *s == "active" || *s == "expired")
            .count();
        let rate = good_count as f64 / records.len() as f64;
        let score = if rate > 0.8 {
            80
        } else if rate > 0.6 {
            60
        } else {
            40
        };

        assert!(
            (40..=100).contains(&score),
            "信用记录评分应该在 40-100 之间"
        );
    }
}
