    use super::*;
#[cfg(test)]
mod tests {

    /// M8 测试：parse_date 正确解析完整日期时间
    #[test]
    fn test_parse_date_full_datetime() {
        let s = Some("2026-07-11T08:00:00Z".to_string());
        let result = parse_date(&s).unwrap();
        assert!(result.is_some());
    }

    /// M8 测试：parse_date 正确解析日期字符串（补充 00:00:00 UTC）
    #[test]
    fn test_parse_date_date_only() {
        let s = Some("2026-07-11".to_string());
        let result = parse_date(&s).unwrap();
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-07-11");
    }

    /// M8 测试：parse_date None 返回 None
    #[test]
    fn test_parse_date_none() {
        let result = parse_date(&None).unwrap();
        assert!(result.is_none());
    }

    /// M8 测试：parse_date 无效格式返回错误
    #[test]
    fn test_parse_date_invalid() {
        let s = Some("not-a-date".to_string());
        assert!(parse_date(&s).is_err());
    }
}