//! 数据权限 Handler 单元测试
//!
//! 覆盖目标：
//! - validate_custom_condition_safe SQL 注入防御纯函数（6 个分支）

use bingxi_backend::handlers::data_permission_handler::validate_custom_condition_safe;
use serde_json::{Value, json};

/// 场景：Value::Null 应通过校验（无自定义条件）
#[test]
fn test_validate_custom_condition_nulltg() {
    let result = validate_custom_condition_safe(&Value::Null);
    assert!(result.is_ok(), "null 值应通过校验");
}

/// 场景：空对象 {} 应通过校验（无字段需要检查）
#[test]
fn test_validate_custom_conditionkdxtg() {
    let result = validate_custom_condition_safe(&json!({}));
    assert!(result.is_ok(), "空对象应通过校验");
}

/// 场景：合法字段名（小写+下划线+数字）+ 合法值类型（数字/字符串/bool/null）应通过
#[test]
fn test_validate_custom_conditionhfdxtg() {
    let cond = json!({
        "field1": 123,
        "field2": "value",
        "field_3": true,
        "field4": null
    });
    let result = validate_custom_condition_safe(&cond);
    assert!(result.is_ok(), "合法对象应通过校验");
}

/// 场景：字段名含大写字母（如 "FieldName"）应被拒绝
#[test]
fn test_validate_custom_conditionjjdxzdm() {
    let cond = json!({"FieldName": 123});
    let result = validate_custom_condition_safe(&cond);
    assert!(result.is_err(), "含大写字母的字段名应被拒绝");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("字段名非法"),
        "错误消息应包含'字段名非法'，实际：{}",
        msg
    );
}

/// 场景：序列化后包含 UNION/SELECT/DROP 等 FORBIDDEN 关键字应被拒绝
#[test]
fn test_validate_custom_conditionjjsqlgjz() {
    // 字符串值中包含 UNION（序列化后大写匹配）
    let cond = json!({"field": "UNION SELECT"});
    let result = validate_custom_condition_safe(&cond);
    assert!(result.is_err(), "包含 SQL 关键字应被拒绝");

    // 包含分号
    let cond = json!({"field": "value;"});
    let result = validate_custom_condition_safe(&cond);
    assert!(result.is_err(), "包含分号应被拒绝");
}

/// 场景：字符串值包含单引号/双引号应被拒绝（防 SQL 注入）
#[test]
fn test_validate_custom_conditionjjzfchyh() {
    // 单引号
    let cond = json!({"field": "value'with'quotes"});
    let result = validate_custom_condition_safe(&cond);
    assert!(result.is_err(), "含单引号的字符串值应被拒绝");

    // 双引号
    let cond = json!({"field": "value\"with\"quotes"});
    let result = validate_custom_condition_safe(&cond);
    assert!(result.is_err(), "含双引号的字符串值应被拒绝");
}

// ===== 未覆盖分支补锁 =====
// 头注承诺"6 个分支"，但按实现（data_permission_handler.rs:36-92）逐分支对照后
// 存在**未钉分支**：非标量容器值（数组/嵌套对象）、标量顶层输入、空字段名、
// 超长字段名、以及"特殊字符检查漏网但深度禁词网兜住"的组合形态。逐条钉住
// （只收紧、不放宽；断言一律看 error_code 机器码，不比对文案原文）。

/// 断言拒绝且落在校验族机器码（VALIDATION_ERROR），不比对 message 原文
fn assert_rejected_as_validation(result: Result<(), bingxi_backend::utils::error::AppError>) {
    let err = result.expect_err("非法 custom_condition 必须返回 Err（fail-closed）");
    assert_eq!(
        err.error_code(),
        "VALIDATION_ERROR",
        "custom_condition 拒绝必须归校验族，实得 {}",
        err.error_code()
    );
}

/// 实现 :66-70 的 `_` 兜底分支：数组/嵌套对象等非标量值必须是非法（上方 6 分支用例
/// 只覆盖合法标量；容器值形态若被静默放行将拼进 raw SQL，由本条钉住）
#[test]
fn test_validate_custom_condition_rejects_container_values() {
    assert_rejected_as_validation(validate_custom_condition_safe(&json!({"field": [1, 2]})));
    assert_rejected_as_validation(validate_custom_condition_safe(&json!({
        "field": {"nested": 1}
    })));
}

/// 实现 :73-77：顶层标量（非对象、非 null）必须拒——null 放行是"无条件"语义，
/// 标量没有键值形态，不得当空条件静默通过
#[test]
fn test_validate_custom_condition_rejects_non_object_top_level() {
    assert_rejected_as_validation(validate_custom_condition_safe(&json!(42)));
    assert_rejected_as_validation(validate_custom_condition_safe(&json!("x")));
    assert_rejected_as_validation(validate_custom_condition_safe(&json!([1, 2])));
}

/// 实现 :44-49 字段名合法集的边界：空串与 >64 长度都是非法（原用例只测了
/// 大写字母一个违例形态）
#[test]
fn test_validate_custom_condition_rejects_empty_and_oversize_field_names() {
    assert_rejected_as_validation(validate_custom_condition_safe(&json!({"": 1})));
    // 动态键用 Map 显式构造（json! 对表达式键的形态不作赌，保证编译契约）
    let mut oversize = serde_json::Map::new();
    oversize.insert("a".repeat(65), json!(1));
    assert_rejected_as_validation(validate_custom_condition_safe(&Value::Object(oversize)));
    // 恰 64 合法（含小写/数字/下划线词表内）——边界另一侧同样必须钉住，
    // 防止未来把上限误改成 63 或 65
    let mut ok = serde_json::Map::new();
    ok.insert("b".repeat(64), json!(1));
    assert!(
        validate_custom_condition_safe(&Value::Object(ok)).is_ok(),
        "64 长度合法字段名不得被误拒"
    );
}

/// 深度禁词网（实现 :80-90）独立兜底：字符串值不含单双引号/分号、字段名合法，
/// 但序列化后命中 FORBIDDEN（"--"、"/*"）——第一道特殊字符检查漏网时禁词网必须拦；
/// 大小写混合输入也必须被 to_uppercase 归一后命中（原用例只测了大写 UNION）
#[test]
fn test_validate_custom_condition_deep_keyword_net_catches_lowercase() {
    assert_rejected_as_validation(validate_custom_condition_safe(&json!({
        "field": "a--b"
    })));
    assert_rejected_as_validation(validate_custom_condition_safe(&json!({
        "field": "/*x"
    })));
    assert_rejected_as_validation(validate_custom_condition_safe(&json!({
        "field": "union select 1"
    })));
}

/// 合法形态的"正形"边界：值域四标量（数字/字符串/bool/null）+ 键词表
/// （小写/数字/下划线）组合必须放行——补锁违例族后同步钉住不误伤合法入参
/// （非法值族与合法正形同批收紧，防"越收越紧把合法配置全毙了"的假严）
#[test]
fn test_validate_custom_condition_accepts_all_four_scalar_value_kinds() {
    assert!(
        validate_custom_condition_safe(&json!({
            "n": 1,
            "s": "plain_value",
            "b": false,
            "nul": null
        }))
        .is_ok()
    );
}
