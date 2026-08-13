use bingxi_backend::services::report_subscription_service::*;
use bingxi_backend::utils::validation::is_valid_email;


// ========== 缺陷 2.2 补充：订阅权限校验 — 邮箱格式校验测试 ==========

/// 合法邮箱应通过校验
#[test]
fn test_is_valid_email_hefa_youx() {
    assert!(is_valid_email("user@example.com"));
    assert!(is_valid_email("john.doe@company.org"));
    assert!(is_valid_email("test+tag@domain.co.uk"));
    assert!(is_valid_email("a@b.cn"));
}

/// 缺少 @ 符号应拒绝
#[test]
fn test_is_valid_email_queshao_at() {
    assert!(!is_valid_email("userexample.com"));
    assert!(!is_valid_email("user"));
}

/// 多个 @ 符号应拒绝
#[test]
fn test_is_valid_email_duoge_at() {
    assert!(!is_valid_email("user@@example.com"));
    assert!(!is_valid_email("user@ex@ample.com"));
}

/// 空用户名应拒绝
#[test]
fn test_is_valid_email_kong_yhm() {
    assert!(!is_valid_email("@example.com"));
}

/// 空域名应拒绝
#[test]
fn test_is_valid_email_kong_ym() {
    assert!(!is_valid_email("user@"));
}

/// 域名缺少点号应拒绝
#[test]
fn test_is_valid_email_ym_qd_dh() {
    assert!(!is_valid_email("user@localhost"));
    assert!(!is_valid_email("user@example"));
}

/// 域名以点号开头或结尾应拒绝
#[test]
fn test_is_valid_email_ym_dhkg() {
    assert!(!is_valid_email("user@.example.com"));
    assert!(!is_valid_email("user@example.com."));
}

/// 空字符串应拒绝
#[test]
fn test_is_valid_email_kong_zfc() {
    assert!(!is_valid_email(""));
}

// ========== 缺陷 2.3 补充：重试退避间隔测试 ==========

/// 第 1 次重试（retry_count=0）应为 60 秒（1 分钟）
#[test]
fn test_backoff_seconds_di_yi_ci() {
    assert_eq!(backoff_seconds(0), 60);
}

/// 第 2 次重试（retry_count=1）应为 300 秒（5 分钟）
#[test]
fn test_backoff_seconds_di_er_ci() {
    assert_eq!(backoff_seconds(1), 300);
}

/// 第 3 次及以后（retry_count>=2）应为 1800 秒（30 分钟）
#[test]
fn test_backoff_seconds_di_san_ci_yys() {
    assert_eq!(backoff_seconds(2), 1800);
    assert_eq!(backoff_seconds(3), 1800);
    assert_eq!(backoff_seconds(100), 1800);
}

/// 退避间隔应严格递增（指数退避语义）
#[test]
fn test_backoff_seconds_yg_dz() {
    assert!(backoff_seconds(0) < backoff_seconds(1));
    assert!(backoff_seconds(1) < backoff_seconds(2));
}

/// DEFAULT_MAX_RETRIES 常量应为 3
#[test]
fn test_default_max_retries_val() {
    assert_eq!(DEFAULT_MAX_RETRIES, 3);
}