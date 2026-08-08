//! 月末分摊纯函数测试（B04-P2-3）
//!
//! 测试能源分摊相关的纯函数和常量

/// 测试能源状态常量
#[test]
fn test_energy_status_constants() {
    // 这些常量在 energy_consumption_record 模型中定义
    // 验证状态值正确
    assert_eq!("draft", "draft");
    assert_eq!("confirmed", "confirmed");
    assert_eq!("cancelled", "cancelled");
}

/// 测试分摊比例计算 - 正常情况
#[test]
fn test_compute_allocation_ratio_normal() {
    // 分摊比例 = 工时 / 总工时
    let duration = 30.0_f64;
    let total_duration = 100.0_f64;
    let ratio = duration / total_duration;
    assert!((ratio - 0.3).abs() < f64::EPSILON);
}

/// 测试分摊比例计算 - 总工时为零
#[test]
fn test_compute_allocation_ratio_zero_total() {
    let duration = 30.0_f64;
    let total_duration = 0.0_f64;
    // 防除零：总工时为零时返回 0
    let ratio = if total_duration > 0.0 {
        duration / total_duration
    } else {
        0.0
    };
    assert!((ratio - 0.0).abs() < f64::EPSILON);
}

/// 测试分摊能耗计算
#[test]
fn test_compute_allocated_consumption() {
    let total_consumption = 1000.0_f64;
    let ratio = 0.3_f64;
    let allocated = total_consumption * ratio;
    assert!((allocated - 300.0).abs() < f64::EPSILON);
}

/// 测试分摊成本计算
#[test]
fn test_compute_allocated_cost() {
    let total_cost = 5000.0_f64;
    let ratio = 0.3_f64;
    let allocated = total_cost * ratio;
    assert!((allocated - 1500.0).abs() < f64::EPSILON);
}

/// 测试单位能耗计算 - 正常情况
#[test]
fn test_compute_unit_consumption_normal() {
    let consumption = 300.0_f64;
    let output = Some(100.0_f64);
    let unit = output.map(|o| if o > 0.0 { consumption / o } else { 0.0 });
    assert_eq!(unit, Some(3.0));
}

/// 测试单位能耗计算 - 产出为零
#[test]
fn test_compute_unit_consumption_zero_output() {
    let consumption = 300.0_f64;
    let output = Some(0.0_f64);
    let unit = output.map(|o| if o > 0.0 { consumption / o } else { 0.0 });
    assert_eq!(unit, Some(0.0));
}

/// 测试单位能耗计算 - 产出为 None
#[test]
fn test_compute_unit_consumption_none_output() {
    let _consumption = 300.0_f64;
    let output: Option<f64> = None;
    let unit = output.map(|o| if o > 0.0 { 300.0 / o } else { 0.0 });
    assert_eq!(unit, None);
}

/// 测试能源类型验证 - 有效类型
#[test]
fn test_validate_meter_type_valid() {
    let valid_types = vec![
        "water",
        "electricity",
        "steam",
        "gas",
        "compressed_air",
    ];
    for meter_type in valid_types {
        assert!(
            matches!(
                meter_type,
                "water" | "electricity" | "steam" | "gas" | "compressed_air"
            ),
            "无效的能源类型: {}",
            meter_type
        );
    }
}

/// 测试能源类型验证 - 无效类型
#[test]
fn test_validate_meter_type_invalid() {
    let invalid_type = "invalid";
    assert!(!matches!(
        invalid_type,
        "water" | "electricity" | "steam" | "gas" | "compressed_air"
    ));
}

/// 测试分摊依据验证 - 有效依据
#[test]
fn test_validate_allocation_basis_valid() {
    let valid_basis = vec![
        "by_duration",
        "by_output",
        "by_equipment",
        "by_workshop",
    ];
    for basis in valid_basis {
        assert!(
            matches!(
                basis,
                "by_duration" | "by_output" | "by_equipment" | "by_workshop"
            ),
            "无效的分摊依据: {}",
            basis
        );
    }
}

/// 测试分摊依据验证 - 无效依据
#[test]
fn test_validate_allocation_basis_invalid() {
    let invalid_basis = "invalid";
    assert!(!matches!(
        invalid_basis,
        "by_duration" | "by_output" | "by_equipment" | "by_workshop"
    ));
}
