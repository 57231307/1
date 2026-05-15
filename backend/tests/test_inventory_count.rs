//! 库存盘点模块单元测试

#[test]
fn test_count_difference_calculation() {
    let system_qty = 100;
    let actual_qty = 95;
    let diff_qty = actual_qty - system_qty;
    
    assert_eq!(diff_qty, -5);
}

#[test]
fn test_count_variance_detection() {
    let test_cases = vec![
        (100, 100, false),  // 无差异
        (100, 95, true),    // 盘亏
        (100, 105, true),   // 盘盈
        (0, 0, false),      // 零库存
    ];
    
    for (system, actual, has_variance) in test_cases {
        let variance = system != actual;
        assert_eq!(variance, has_variance, "系统：{}, 实际：{}", system, actual);
    }
}

#[test]
fn test_count_status_transition() {
    let valid_transitions = vec![
        ("draft", "approved"),
        ("approved", "in_progress"),
        ("in_progress", "completed"),
    ];
    
    for (from, to) in valid_transitions {
        assert!(is_valid_transition(from, to), "无效的状态转换：{} -> {}", from, to);
    }
}

fn is_valid_transition(from: &str, to: &str) -> bool {
    matches!((from, to),
        ("draft", "approved") |
        ("approved", "in_progress") |
        ("in_progress", "completed")
    )
}
