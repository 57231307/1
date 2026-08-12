#[cfg(test)]
mod tests {
use bingxi_backend::cli::admin::*;


    /// H-2 测试（v9 复审）：未提供密码时返回错误
    #[test]
    fn test_read_password_no_source() {
        unsafe { std::env::remove_var("BINGXI_ADMIN_PASSWORD"); }
        let result = read_password(false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未提供密码"));
    }

    /// H-2 测试（v9 复审）：从环境变量读取密码
    #[test]
    fn test_read_password_from_env() {
        unsafe { std::env::set_var("BINGXI_ADMIN_PASSWORD", "test_secret_123"); }
        let result = read_password(false);
        assert_eq!(result.unwrap(), "test_secret_123");
        unsafe { std::env::remove_var("BINGXI_ADMIN_PASSWORD"); }
    }

    /// H-2 测试（v9 复审）：环境变量优先于 stdin
    #[test]
    fn test_read_password_env_takes_precedence() {
        unsafe { std::env::set_var("BINGXI_ADMIN_PASSWORD", "env_password"); }
        // 即使 from_stdin=true，环境变量也优先
        let result = read_password(true);
        assert_eq!(result.unwrap(), "env_password");
        unsafe { std::env::remove_var("BINGXI_ADMIN_PASSWORD"); }
    }

    /// H-2 测试（v9 复审）：空环境变量被忽略
    #[test]
    fn test_read_password_empty_env_ignored() {
        unsafe { std::env::set_var("BINGXI_ADMIN_PASSWORD", ""); }
        let result = read_password(false);
        assert!(result.is_err());
        unsafe { std::env::remove_var("BINGXI_ADMIN_PASSWORD"); }
    }
}