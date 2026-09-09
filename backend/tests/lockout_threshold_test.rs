//! 锁定阈值常量断言测试
//!
//! P1.3 锁定阈值 5→9：验证 IP 维度 9、全局维度 18（MAX_FAILED_ATTEMPTS * 2）。
//! 纯常量测试，不依赖 DB/HTTP。

use bingxi_backend::handlers::auth_handler::MAX_FAILED_ATTEMPTS as AUTH_MAX;
use bingxi_backend::handlers::login_security_handler::MAX_FAILED_ATTEMPTS as LOGIN_MAX;

/// IP 维度锁定阈值 = 9
#[test]
fn test_ip_lockout_threshold_is_9() {
    assert_eq!(AUTH_MAX, 9, "auth_handler MAX_FAILED_ATTEMPTS 应为 9");
    assert_eq!(
        LOGIN_MAX, 9,
        "login_security_handler MAX_FAILED_ATTEMPTS 应为 9"
    );
}

/// 全局锁定阈值 = IP 维度 × 2 = 18
#[test]
fn test_global_lockout_threshold_is_18() {
    let global = AUTH_MAX * 2;
    assert_eq!(global, 18, "全局锁定阈值应为 18（IP 9 × 2）");
}

/// 两处常量一致
#[test]
fn test_two_handlers_threshold_consistent() {
    assert_eq!(AUTH_MAX, LOGIN_MAX, "两处 MAX_FAILED_ATTEMPTS 应一致");
}
