    use bingxi_backend::cli::util::*;
#[cfg(test)]
mod tests {

    /// M8 测试：get_env_file_path 默认返回 /etc/bingxi/.env
    #[test]
    fn test_get_env_file_path_default() {
        unsafe { std::env::remove_var("BINGXI_ENV_FILE"); }
        assert_eq!(get_env_file_path(), "/etc/bingxi/.env");
    }

    /// M8 测试：get_env_file_path 从环境变量读取
    #[test]
    fn test_get_env_file_path_from_env() {
        unsafe { std::env::set_var("BINGXI_ENV_FILE", "/custom/path/.env"); }
        assert_eq!(get_env_file_path(), "/custom/path/.env");
        unsafe { std::env::remove_var("BINGXI_ENV_FILE"); }
    }

    /// M8 测试：get_systemd_dir 默认返回 /etc/systemd/system
    #[test]
    fn test_get_systemd_dir_default() {
        unsafe { std::env::remove_var("BINGXI_SYSTEMD_DIR"); }
        assert_eq!(get_systemd_dir(), "/etc/systemd/system");
    }

    /// M8 测试：get_systemd_dir 从环境变量读取
    #[test]
    fn test_get_systemd_dir_from_env() {
        unsafe { std::env::set_var("BINGXI_SYSTEMD_DIR", "/custom/systemd"); }
        assert_eq!(get_systemd_dir(), "/custom/systemd");
        unsafe { std::env::remove_var("BINGXI_SYSTEMD_DIR"); }
    }

    // 批次 322 v9 复审低危修复：validate_dir_recursive 和 validate_extracted_paths 的
    // 单元测试已迁移到共享模块 utils::path_validator，此处不再重复维护。
}