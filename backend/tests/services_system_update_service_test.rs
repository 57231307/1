use bingxi_backend::services::system_update_service::*;

/// M8 测试：validate_download_url 合法 GitHub URL 通过
#[test]
fn test_validate_download_url_valid() {
    assert!(
        validate_download_url("https://github.com/57231307/1/releases/download/v1.0.0/pkg.zip")
            .is_ok()
    );
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

// ============ 三段(MD 折叠)↔四段 跨格式比较（假阴性回归锁定） ============

/// 原假阴性回归：Cargo 三段 current `2026.929.1111`（Sep29 11:11）vs tag 四段 Oct1 latest
/// `2026.10.1.0930` —— 逐段数值比会误判 current 更新（929 > 10）。归一反解后 Oct1 新于 Sep29。
#[test]
fn test_compare_versions_cross_format_md_folded_newer() {
    let svc = SystemUpdateService::new();
    assert!(svc.compare_versions("2026.929.1111", "2026.10.1.0930"));
}

/// 反解后相等：三段 `2026.929.1111` 与四段 `2026.9.29.1111` 是同一版本的两种编码，latest 不严格大于 → false
#[test]
fn test_compare_versions_cross_format_equal_after_expand() {
    let svc = SystemUpdateService::new();
    assert!(!svc.compare_versions("2026.929.1111", "2026.9.29.1111"));
}

/// 同日更晚分钟：三段 `2026.929.1111`（Sep29 11:11）vs 四段 `2026.9.29.1112`（Sep29 11:12）→ latest 更新
#[test]
fn test_compare_versions_cross_format_same_day_later_minute() {
    let svc = SystemUpdateService::new();
    assert!(svc.compare_versions("2026.929.1111", "2026.9.29.1112"));
}

/// 1 月边界 MD 反解：三段 `2026.101.0930`（Jan1 09:30）vs 四段 `2026.1.1.1000`（Jan1 10:00）→ latest 更新
#[test]
fn test_compare_versions_cross_format_january_boundary() {
    let svc = SystemUpdateService::new();
    assert!(svc.compare_versions("2026.101.0930", "2026.1.1.1000"));
}

/// 同四段（注入/tag 格式）直接逐段比：Sep29 vs Oct1 → latest 更新
#[test]
fn test_compare_versions_both_four_segment_calver() {
    let svc = SystemUpdateService::new();
    assert!(svc.compare_versions("2026.9.29.1111", "2026.10.1.0930"));
}

/// 排序归一：三段/四段混排列表按版本降序（锁 list_local_releases 排序口径）
#[test]
fn test_compare_versions_for_sort_cross_format_mixed() {
    let svc = SystemUpdateService::new();
    // 三段 Sep29 vs 四段 Oct1：Oct1 更新 → 降序里 Oct1 在前，compare_versions_for_sort 应返回 Greater
    assert_eq!(
        svc.compare_versions_for_sort("2026.929.1111", "2026.10.1.0930"),
        std::cmp::Ordering::Greater,
        "三段 Sep29 排四段 Oct1 之后（降序：较新在前）"
    );
    // 三段与四段同刻：反解后相等 → Equal
    assert_eq!(
        svc.compare_versions_for_sort("2026.929.1111", "2026.9.29.1111"),
        std::cmp::Ordering::Equal,
        "反解后相等版本排序 Equal"
    );
    // 完整混排列降序，首元素必为最新版本
    let mut versions: Vec<String> = ["2026.9.29.1111", "2026.929.1111", "2026.10.1.0930"]
        .into_iter()
        .map(str::to_string)
        .collect();
    versions.sort_by(|a, b| svc.compare_versions_for_sort(a, b));
    // 首元素必为最新版本（Oct1 新于 Sep29）
    assert_eq!(versions.first().map(|s| s.as_str()), Some("2026.10.1.0930"));
    // 两个 Sep29 变体反解后相等，同属最旧组（相对顺序不定，按字典序归一后断言集合一致）
    let mut oldest: Vec<String> = versions[1..].to_vec();
    oldest.sort();
    assert_eq!(
        oldest,
        vec!["2026.9.29.1111".to_string(), "2026.929.1111".to_string()]
    );
}

/// 归一/反解边界：to_calver_quad 对非法 MD 折叠（第 2 段反解出 >12 月 / 0 日）返回 None，不崩
#[test]
fn test_to_calver_quad_invalid_md() {
    // MD=1399 → 月 13 非法
    assert_eq!(to_calver_quad(&[2026, 1399, 1111]), None);
    // MD=900 → 月 9 合法但日 0 非法
    assert_eq!(to_calver_quad(&[2026, 900, 1111]), None);
    // MD=0 → 月 0 非法
    assert_eq!(to_calver_quad(&[2026, 0, 1111]), None);
    // 非本项目 CalVer 年份（< 2000）的三段不臆测
    assert_eq!(to_calver_quad(&[1999, 929, 1111]), None);
    // 段数不足 / 超出
    assert_eq!(to_calver_quad(&[2026, 9]), None);
    assert_eq!(to_calver_quad(&[2026, 9, 29, 1111, 5]), None);
    // 合法反解 + 四段透传
    assert_eq!(
        to_calver_quad(&[2026, 929, 1111]),
        Some([2026, 9, 29, 1111])
    );
    assert_eq!(
        to_calver_quad(&[2026, 9, 29, 1111]),
        Some([2026, 9, 29, 1111])
    );
}

/// 跨格式不可判定：三段 MD 非法且配四段 latest 时不崩，退回 element-wise 补 0 比较
#[test]
fn test_compare_versions_cross_format_invalid_md_does_not_panic() {
    let svc = SystemUpdateService::new();
    // current 三段 [2026,1399,1111] 反解非法 → element-wise：idx1 1399 > 10 → latest 不更新 → false
    assert!(!svc.compare_versions("2026.1399.1111", "2026.10.1.0930"));
}
