#[cfg(test)]
mod tests {
    use bingxi_backend::utils::unwrap_safe::*;
    #[test]
    fn test_dec_macro() {
        // P9-1: 用宏替代散落的 expect，验证宏行为
        let v = dec!(1000.0);
        assert_eq!(v.to_string(), "1000");
    }

    #[test]
    fn test_int_macro() {
        let v: i64 = int!("42");
        assert_eq!(v, 42);
    }

    #[test]
    fn test_s_macro() {
        let v: String = s!("hello-p9-1");
        assert_eq!(v, "hello-p9-1");
    }
}