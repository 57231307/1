use bingxi_backend::middleware::audit_context::CLIENT_IP_UNRESOLVED;
use bingxi_backend::utils::cache::*;
use std::time::Duration;

const SESSION_A: &str = "session-aaa";
const SESSION_B: &str = "session-bbb";
const IP_WIFI: &str = "203.0.113.10";
const IP_CELL: &str = "198.51.100.99";

/// 正向基线：用户与会话一致且 IP 未变时消费成功，并同步清理用户索引
#[test]
fn test_consume_with_matching_binding_and_same_ip_returns_ok() {
    let cache = AppCache::new();
    let token = "test-csrf-token-001".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::Ok,
        "绑定与 IP 都一致应返回 Ok，实际: {:?}",
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A))
    );
    assert!(
        !cache.csrf_user_index.contains_key(&42),
        "consume 后反向索引应移除 user_id=42"
    );
}

/// 正向核心改动：同一会话同一用户下 IP 变化不再拒绝，返回 OkIpDrift 并带出漂移前的 IP
#[test]
fn test_ip_change_within_same_session_consumes_as_drift_not_rejection() {
    let cache = AppCache::new();
    let token = "test-csrf-token-drift".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    let result = cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_A));
    assert_eq!(
        result,
        CsrfConsumeResult::OkIpDrift {
            previous_ip: IP_WIFI.to_string()
        },
        "IP 漂移必须是可放行形态（携带漂移前 IP 供留痕），不得是拒绝形态，实际: {:?}",
        result
    );
}

/// 正向：漂移后旧 Token 不立刻作废。宽限窗口内同一会话的在途请求仍可拿它完成消费，
/// 这是"一次换网把整批并发写请求打成 403"不重演的前提。
#[test]
fn test_drift_keeps_old_token_consumable_within_grace_window() {
    let cache = AppCache::new();
    let token = "test-csrf-token-grace".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    let first = cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_A));
    assert!(
        matches!(first, CsrfConsumeResult::OkIpDrift { .. }),
        "第一次请求应判为漂移，实际: {:?}",
        first
    );

    // 在途并发请求仍携旧 token（同一新出口 IP）：应正常消费而不是 NotFound
    let second = cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_A));
    assert_eq!(
        second,
        CsrfConsumeResult::Ok,
        "宽限窗口内旧 Token 必须仍可消费（副本已改绑到新 IP），实际: {:?}",
        second
    );
}

/// 正向：宽限窗口是有界的。窗口过后旧 Token 彻底消失，不给被窃 Token 拉长可用期。
#[test]
fn test_drift_grace_copy_expires_after_window() {
    let cache = AppCache::new();
    let token = "test-csrf-token-grace-expiry".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    cache.consume_csrf_token_with_grace(
        &token,
        IP_CELL,
        Some(42),
        Some(SESSION_A),
        Duration::from_millis(30),
    );
    std::thread::sleep(Duration::from_millis(120));

    assert_eq!(
        cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::NotFound,
        "宽限窗口结束后旧 Token 必须已失效，不能变成常驻条目"
    );
}

/// 反向：宽限副本仍受绑定约束。跨用户或跨会话拿它一样被拒，且必须被强制轮换清除。
#[test]
fn test_drift_grace_copy_is_still_bound_and_cleared_by_rotation() {
    let cache = AppCache::new();
    let token = "test-csrf-token-grace-binding".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );
    cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_A));

    // 用户索引必须登记宽限副本，否则重新登录/刷新清不掉它
    assert!(
        cache
            .csrf_user_index
            .get(&42)
            .map(|set| set.contains(&token))
            .unwrap_or(false),
        "宽限副本必须进用户索引，才能被强制轮换整集清除"
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_CELL, Some(43), Some(SESSION_A)),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::UserMismatch),
        "宽限副本不得被别的用户消费"
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_B)),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::SessionMismatch),
        "宽限副本不得被同用户的别的会话消费"
    );

    // 强制轮换（登录/刷新链路）把副本一并清除
    assert!(
        cache.clear_old_csrf_token_for_user(42),
        "宽限副本应随强制轮换被清除"
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_CELL, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::NotFound,
        "强制轮换后副本必须不存在"
    );
}

/// 反向：别的用户的 Token 必须拒，且拒绝不得消耗掉原主的有效 Token（防 DoS 探测）
#[test]
fn test_other_users_token_is_rejected_and_not_consumed() {
    let cache = AppCache::new();
    let token = "test-csrf-token-owner".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(43), Some(SESSION_A)),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::UserMismatch),
        "跨用户消费必须被拒"
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::Ok,
        "探测方被拒时不得顺带消耗掉原主的 Token，否则一条反向请求就能把人踢下线"
    );
}

/// 反向：同用户但别的会话的 Token 必须拒（IP 恰好一致也救不了它）
#[test]
fn test_other_session_token_is_rejected_even_with_matching_ip() {
    let cache = AppCache::new();
    let token = "test-csrf-token-session".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_B.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::SessionMismatch),
        "IP 完全一致也不能替代会话核验，防止伪造 X-Forwarded-For 冒充绑定地址"
    );
}

/// 反向：伪造的 X-Forwarded-For 只影响 IP 字符串本身。
/// 攻击方把 IP 伪造成受害者的绑定 IP，仍然因为用户/会话核验不过而被拒；核验只看绑定不看来源。
#[test]
fn test_forged_ip_cannot_bypass_binding_check() {
    let cache = AppCache::new();
    let token = "test-csrf-token-spoof".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    // 攻击方自称 203.0.113.10（与绑定值逐字相同），但它是另一个会话
    let result = cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_B));
    assert_eq!(
        result,
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::SessionMismatch),
        "IP 逐字相等不得被当成会话归属证明"
    );
    assert_eq!(
        cache.consume_csrf_token(&token, "192.0.2.254", Some(7), Some(SESSION_B)),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::UserMismatch),
        "任意伪造 IP 组合下，跨用户仍必拒"
    );
}

/// 反向：请求没有认证上下文时不得消费任何 Token（否则等于把一次性 Token 变成无主凭证）
#[test]
fn test_missing_auth_context_is_rejected() {
    let cache = AppCache::new();
    let token = "test-csrf-token-noauth".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, None, None),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::Unauthenticated),
        "无认证上下文必须拒"
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::Ok,
        "无认证上下文的失败请求不得消耗 Token"
    );
}

/// 反向锁：未绑定会话的 Token（签发时拿不到 JWT 会话）不参与会话比对，但用户维度仍然必核
#[test]
fn test_unbound_session_token_still_enforces_user_binding() {
    let cache = AppCache::new();
    let token = "test-csrf-token-unbound".to_string();
    cache.set_csrf_token(
        token.clone(),
        CSRF_SESSION_UNBOUND.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );

    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::Ok,
        "会话占位值只放开会话维度比对"
    );

    cache.set_csrf_token(
        token.clone(),
        CSRF_SESSION_UNBOUND.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(43), Some(SESSION_A)),
        CsrfConsumeResult::BindingMismatch(CsrfBindingMismatch::UserMismatch),
        "占位会话不得顺带放宽用户核验"
    );
}

/// 既有行为不回退：不存在的 Token 与过期 Token 都是 NotFound
#[test]
fn test_missing_and_expired_tokens_are_not_found() {
    let cache = AppCache::new();
    assert_eq!(
        cache.consume_csrf_token("non-existent-token", IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::NotFound,
        "不存在的 token 应返回 NotFound"
    );

    let expired = "test-csrf-token-expired".to_string();
    cache.set_csrf_token(
        expired.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        Some(Duration::from_millis(10)),
    );
    std::thread::sleep(Duration::from_millis(80));
    assert_eq!(
        cache.consume_csrf_token(&expired, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::NotFound,
        "TTL 过期的 token 必须仍是 NotFound，不得被漂移逻辑救活"
    );
}

/// 既有行为不回退：一次性消费语义（第二次消费是 NotFound 而不是重放成功）
#[test]
fn test_one_time_use_semantics_preserved() {
    let cache = AppCache::new();
    let token = "test-csrf-token-onetime".to_string();
    cache.set_csrf_token(
        token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        42,
        None,
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::Ok
    );
    assert_eq!(
        cache.consume_csrf_token(&token, IP_WIFI, Some(42), Some(SESSION_A)),
        CsrfConsumeResult::NotFound,
        "同会话第二次使用同一 token 必须是 NotFound（重放不得通过）"
    );
}

/// 既有行为不回退：强制轮换清除该用户全部活跃 Token
#[test]
fn test_clear_old_csrf_token_for_user_invalidates_old_token() {
    let cache = AppCache::new();
    let old_token = "old-csrf-token-003".to_string();
    cache.set_csrf_token(
        old_token.clone(),
        SESSION_A.to_string(),
        IP_WIFI.to_string(),
        44,
        None,
    );

    let cleared = cache.clear_old_csrf_token_for_user(44);
    assert!(cleared, "应返回 true（存在旧 token）");

    let r = cache.consume_csrf_token(&old_token, IP_WIFI, Some(44), Some(SESSION_A));
    assert_eq!(
        r,
        CsrfConsumeResult::NotFound,
        "清除后旧 token 应返回 NotFound，实际: {:?}",
        r
    );

    let cleared_none = cache.clear_old_csrf_token_for_user(999);
    assert!(!cleared_none, "无活跃 token 的用户应返回 false");
}

/// 漂移判据本身：占位值与真实 IP 的组合不构成漂移（观测能力差异不是换网）
#[test]
fn test_unresolved_ip_never_counts_as_drift() {
    assert!(
        !csrf_ip_drifted(CLIENT_IP_UNRESOLVED, IP_WIFI),
        "unknown 与真实 IP 之间不得判漂移（登录无反代拿 unknown、后续拿真 IP 的常见组合）"
    );
    assert!(
        !csrf_ip_drifted(IP_WIFI, CLIENT_IP_UNRESOLVED),
        "真实 IP 与 unknown 的反向组合同样不得判漂移"
    );
    assert!(
        !csrf_ip_drifted(CLIENT_IP_UNRESOLVED, CLIENT_IP_UNRESOLVED),
        "两侧都是占位值时不是漂移"
    );
    assert!(
        !csrf_ip_drifted("", IP_WIFI),
        "空串是缺失观测而不是换网"
    );
}

/// 漂移判据本身：纯格式差异不算换址（IPv4 与 v4-mapped IPv6 写法、大小写、前后空白）
#[test]
fn test_ip_format_variance_is_not_drift() {
    assert!(
        !csrf_ip_drifted("::ffff:1.2.3.4", "1.2.3.4"),
        "IPv4 映射 IPv6 写法与 IPv4 是同一地址，不能判成换网"
    );
    assert!(
        !csrf_ip_drifted("1.2.3.4", "::FFFF:1.2.3.4"),
        "规范化比较要覆盖反方向且大小写不敏感"
    );
    assert!(
        !csrf_ip_drifted(" 203.0.113.10 ", "203.0.113.10"),
        "前后空白差异不构成换网"
    );
    assert!(
        !csrf_ip_drifted("2001:db8::1", "2001:DB8::1"),
        "IPv6 大小写差异不构成换网"
    );
}

/// 漂移判据本身：真换址必须算漂移（Wi-Fi 与蜂窝切换、DHCP 换址、NAT 出口池轮换）
#[test]
fn test_real_address_change_counts_as_drift() {
    assert!(
        csrf_ip_drifted(IP_WIFI, IP_CELL),
        "两个不同的 IPv4 必须判漂移，否则换网留痕恒不触发"
    );
    assert!(
        csrf_ip_drifted("2001:db8::1", "2001:db8::2"),
        "IPv6 地址变化必须判漂移"
    );
    assert!(
        csrf_ip_drifted("203.0.113.10", "2001:db8::1"),
        "协议族切换（v4 到 v6）必须判漂移"
    );
}

/// 核验判据本身的次序锁：用户维度先于会话维度，两类不符都必须给出可归因的分类
#[test]
fn test_binding_check_reports_the_most_specific_mismatch() {
    let binding = CsrfTokenBinding {
        session_id: SESSION_A.to_string(),
        ip_address: IP_WIFI.to_string(),
        user_id: 42,
    };
    assert_eq!(
        csrf_binding_mismatch(&binding, None, None),
        Some(CsrfBindingMismatch::Unauthenticated),
        "拿不到认证上下文时归因为 Unauthenticated"
    );
    assert_eq!(
        csrf_binding_mismatch(&binding, Some(43), Some(SESSION_B)),
        Some(CsrfBindingMismatch::UserMismatch),
        "用户与会话同时不符时先归因用户维度（用户是更强的身份边界）"
    );
    assert_eq!(
        csrf_binding_mismatch(&binding, Some(42), Some(SESSION_B)),
        Some(CsrfBindingMismatch::SessionMismatch),
        "用户一致而会话不符归因会话维度"
    );
    assert_eq!(
        csrf_binding_mismatch(&binding, Some(42), Some(SESSION_A)),
        None,
        "两项一致时不得误报"
    );
}

/// 判定器自证：IP 全等比较被彻底移出消费链路（把这条注释掉的写法恢复回去必须判红）
#[test]
fn test_ip_equality_is_not_a_rejection_basis_in_cache_source() {
    let src = include_str!("../src/utils/cache.rs");
    assert!(
        !src.contains("IpMismatch"),
        "cache.rs 仍保留 IpMismatch 拒绝形态：IP 差异被改回拒绝条件"
    );
    assert!(
        !src.contains("bound_ip != client_ip") && !src.contains("ip_address.as_str() != client_ip"),
        "cache.rs 里出现了把绑定 IP 与请求 IP 做全等比较后拒绝的写法"
    );
    assert!(
        src.contains("CsrfConsumeResult::OkIpDrift"),
        "cache.rs 必须把 IP 漂移建模为可放行形态"
    );
    assert!(
        src.contains("CSRF_IP_DRIFT_GRACE_SECS"),
        "cache.rs 必须有界宽限窗口，不许把旧 token 立刻作废"
    );
    assert!(
        src.contains("CsrfBindingMismatch::UserMismatch")
            && src.contains("CsrfBindingMismatch::SessionMismatch"),
        "拒绝依据必须同时包含用户与会话两个维度，否则降级 IP 就是纯放宽"
    );
}
