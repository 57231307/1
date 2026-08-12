#[cfg(test)]
mod tests {
    use super::*;

    /// M8 测试：合法币种码应通过校验
    #[test]
    fn test_validate_currency_code_valid() {
        assert!(validate_currency_code("USD").is_ok());
        assert!(validate_currency_code("EUR").is_ok());
        assert!(validate_currency_code("CNY").is_ok());
    }

    /// M8 测试：长度不正确应被拒绝
    #[test]
    fn test_validate_currency_code_invalid_length() {
        assert!(validate_currency_code("US").is_err());
        assert!(validate_currency_code("USDD").is_err());
        assert!(validate_currency_code("").is_err());
    }

    /// M8 测试：非大写字母应被拒绝
    #[test]
    fn test_validate_currency_code_not_uppercase() {
        assert!(validate_currency_code("usd").is_err());
        assert!(validate_currency_code("Usd").is_err());
        assert!(validate_currency_code("US1").is_err());
        assert!(validate_currency_code("U-S").is_err());
    }

    /// L5 测试：不在白名单中的 3 字母组合应被拒绝
    #[test]
    fn test_validate_currency_code_not_in_whitelist() {
        assert!(validate_currency_code("ZZZ").is_err());
        assert!(validate_currency_code("XXX").is_err());
        assert!(validate_currency_code("ABC").is_err());
    }
}