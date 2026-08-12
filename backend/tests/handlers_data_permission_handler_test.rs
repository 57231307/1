#[cfg(test)]
mod tests {
    //! 数据权限 Handler 单元测试（批次 394 补测）
    //!
    //! 覆盖目标：
    //! - validate_custom_condition_safe SQL 注入防御纯函数（6 个分支）

    use super::*;
    use serde_json::json;

    /// test_validate_custom_condition_nulltg；场景：Value::Null 应通过校验（无自定义条件）
    #[test]
    fn test_validate_custom_condition_nulltg() {
        let result = validate_custom_condition_safe(&Value::Null);
        assert!(result.is_ok(), "null 值应通过校验");
    }

    /// test_validate_custom_conditionkdxtg；场景：空对象 {} 应通过校验（无字段需要检查）
    #[test]
    fn test_validate_custom_conditionkdxtg() {
        let result = validate_custom_condition_safe(&json!({}));
        assert!(result.is_ok(), "空对象应通过校验");
    }

    /// test_validate_custom_conditionhfdxtg；场景：合法字段名（小写+下划线+数字）+ 合法值类型（数字/字符串/bool/null）应通过
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

    /// test_validate_custom_conditionjjdxzdm；场景：字段名含大写字母（如 "FieldName"）应被拒绝
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

    /// test_validate_custom_conditionjjsqlgjz；场景：序列化后包含 UNION/SELECT/DROP 等 FORBIDDEN 关键字应被拒绝
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

    /// test_validate_custom_conditionjjzfchyh；场景：字符串值包含单引号/双引号应被拒绝（防 SQL 注入）
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
}