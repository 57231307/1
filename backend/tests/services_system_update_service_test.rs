#[cfg(test)]
mod tests {
    use super::*;

    /// M8 测试：validate_download_url 合法 GitHub URL 通过
    #[test]
    fn test_validate_download_url_valid() {
        assert!(validate_download_url(
            "https://github.com/57231307/1/releases/download/v1.0.0/pkg.zip"
        )
        .is_ok());
        assert!(validate_download_url("https://objects.githubusercontent.com/assets/123").is_ok());
    }

    /// M8 测试：validate_download_url 非 HTTPS 被拒绝
    #[test]
    fn test_validate_download_url_not_https() {
        assert!(validate_download_url("http://github.com/repo/releases").is_err());
    }

    /// M8 测试：validate_download_url 非允许域名被拒绝
    #[test]
    fn test_validate_download_url_invalid_host() {
        assert!(validate_download_url("https://evil.com/exploit").is_err());
        assert!(validate_download_url("https://169.254.169.254/metadata").is_err());
    }

    /// M8 测试：validate_download_url 无效 URL 被拒绝
    #[test]
    fn test_validate_download_url_invalid_url() {
        assert!(validate_download_url("not-a-url").is_err());
        assert!(validate_download_url("").is_err());
    }

    /// M8 测试：compare_versions 新版本大于旧版本返回 true
    #[test]
    fn test_compare_versions_newer() {
        let svc = SystemUpdateService::new();
        assert!(svc.compare_versions("1.0.0", "1.0.1"));
        assert!(svc.compare_versions("1.0.0", "2.0.0"));
        assert!(svc.compare_versions("2026.7.1", "2026.7.2"));
    }

    /// M8 测试：compare_versions 旧版本大于等于新版本返回 false
    #[test]
    fn test_compare_versions_older_or_equal() {
        let svc = SystemUpdateService::new();
        assert!(!svc.compare_versions("1.0.1", "1.0.0"));
        assert!(!svc.compare_versions("1.0.0", "1.0.0"));
    }

    /// M8 测试：extract_version_from_filename 正确提取版本号
    #[test]
    fn test_extract_version_from_filename() {
        let svc = SystemUpdateService::new();
        assert_eq!(
            svc.extract_version_from_filename("bingxi-erp-1.0.0.zip"),
            Some("1.0.0".to_string())
        );
        assert_eq!(
            svc.extract_version_from_filename("bingxi-erp-2026.7.12.zip"),
            Some("2026.7.12".to_string())
        );
    }

    /// M8 测试：extract_version_from_filename 无效文件名返回 None
    #[test]
    fn test_extract_version_from_filename_invalid() {
        let svc = SystemUpdateService::new();
        assert_eq!(svc.extract_version_from_filename("invalid.zip"), None);
        assert_eq!(svc.extract_version_from_filename("bingxi-erp-.zip"), None);
    }

    /// P0-2 测试（v9 复审）：set_safe_permissions 文件分支应用 0o600 掩码
    #[cfg(unix)]
    #[test]
    fn test_set_safe_permissions_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let temp = std::env::temp_dir().join("bingxi_test_perm_file");
        let _ = std::fs::write(&temp, b"test");
        // 模拟恶意 zip 设置 SUID + SGID + sticky + 全读写（0o7777）
        set_safe_permissions(&temp, 0o7777, false);
        let mode = std::fs::metadata(&temp).unwrap().permissions().mode();
        // 0o7777 & 0o600 = 0o600
        assert_eq!(mode & 0o7777, 0o600, "文件权限应为 0o600，实际 {:#o}", mode);
        let _ = std::fs::remove_file(&temp);
    }

    /// P0-2 测试（v9 复审）：set_safe_permissions 目录分支应用 0o755 掩码
    #[cfg(unix)]
    #[test]
    fn test_set_safe_permissions_dir_mode() {
        use std::os::unix::fs::PermissionsExt;
        let temp = std::env::temp_dir().join("bingxi_test_perm_dir");
        let _ = std::fs::create_dir(&temp);
        // 模拟恶意 zip 设置 SUID + SGID + sticky + 全读写（0o7777）
        set_safe_permissions(&temp, 0o7777, true);
        let mode = std::fs::metadata(&temp).unwrap().permissions().mode();
        // 0o7777 & 0o755 = 0o755
        assert_eq!(mode & 0o7777, 0o755, "目录权限应为 0o755，实际 {:#o}", mode);
        let _ = std::fs::remove_dir(&temp);
    }

    /// M-2 测试（v9 复审）：合法 asset.name 通过校验
    #[test]
    fn test_validate_asset_name_valid() {
        assert!(validate_asset_name("bingxi-erp-1.0.0.zip").is_ok());
        assert!(validate_asset_name("release-2026.7.12.tar.gz").is_ok());
        assert!(validate_asset_name("update_v2.tar.gz").is_ok());
    }

    /// M-2 测试（v9 复审）：路径穿越 asset.name 被拒绝
    #[test]
    fn test_validate_asset_name_path_traversal() {
        assert!(validate_asset_name("../../../etc/cron.d/evil").is_err());
        assert!(validate_asset_name("..\\..\\windows\\evil").is_err());
        assert!(validate_asset_name("/etc/passwd").is_err());
        assert!(validate_asset_name(".hidden").is_err());
        assert!(validate_asset_name("..").is_err());
    }

    /// M-2 测试（v9 复审）：特殊字符 asset.name 被拒绝
    #[test]
    fn test_validate_asset_name_special_chars() {
        assert!(validate_asset_name("file name.zip").is_err()); // 空格
        assert!(validate_asset_name("file;evil.zip").is_err()); // 分号
        assert!(validate_asset_name("file|evil.zip").is_err()); // 管道符
        assert!(validate_asset_name("").is_err()); // 空
    }

    // ============ 批次 322 v9 复审低危修复：parse_version 单元测试 ============

    /// 测试 parse_version 正确解析标准语义版本号
    #[test]
    fn test_parse_version_standard() {
        assert_eq!(parse_version("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("2.0"), vec![2, 0]);
        assert_eq!(parse_version("2026.7.12"), vec![2026, 7, 12]);
    }

    /// 测试 parse_version 解析带预发布标签的版本号（非数字部分被忽略）
    #[test]
    fn test_parse_version_pre_release() {
        assert_eq!(parse_version("1.0.0-beta"), vec![1, 0, 0]);
        assert_eq!(parse_version("2.0.0-rc.1"), vec![2, 0, 0]);
    }

    /// 测试 parse_version 解析空字符串和无效输入
    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("").is_empty());
        assert!(parse_version("abc").is_empty());
        assert_eq!(parse_version("1.a.3"), vec![1, 3]);
    }
}