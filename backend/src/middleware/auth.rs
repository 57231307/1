use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::services::auth_service::AuthService;
use crate::utils::app_state::AppState;
use crate::utils::audit::{self, SecurityEvent};
use crate::utils::cache::Cache;
use crate::utils::request_ext::PublicPathCache;
use crate::utils::response::unauthorized_response;
use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use axum_extra::extract::cookie::{Key, PrivateCookieJar};
use dashmap::DashMap;
use std::sync::OnceLock;
use std::time::Instant;
use tracing::{info, warn};

/// 日志脱敏：截断 Authorization 头值，避免完整 Token 写入日志
///
/// 低危 #4 修复：原实现直接把 `header_val` 拼到 warn 日志中，
/// 可能导致完整 JWT Token 落地到日志文件/聚合系统，违反最小暴露原则。
/// 本函数仅返回格式与长度供排错使用，Token 部分被截断。
///
/// # 参数
/// - `header_val`: 原始 Authorization 头值（可能含 `Bearer xxx...`）
///
/// # 返回
/// - 脱敏后的字符串：仅显示前缀与长度，如 `Bearer abc***(len=143)`
fn mask_auth_header(header_val: &str) -> String {
    // 截取前 12 个字符（包含 "Bearer " 前缀和 Token 前 6 位），剩余长度记入日志
    const PREFIX_KEEP: usize = 12;
    let total_len = header_val.len();
    if total_len <= PREFIX_KEEP {
        // 太短不显示任何字符（防御极端短 header）
        format!("***redacted***(len={})", total_len)
    } else {
        let prefix = &header_val[..PREFIX_KEEP];
        format!("{}***(len={})", prefix, total_len)
    }
}

/// 日志脱敏：用户名 PII 截断
///
/// 低危 #4 修复：warn 级别日志中只保留用户名前 2 字符 + `***`，
/// 避免明文 username 进入日志聚合系统；保留前 2 字符仍可定位用户。
///
/// # 参数
/// - `username`: 原始用户名
///
/// # 返回
/// - 脱敏后的字符串，如 `al***`
fn mask_username(username: &str) -> String {
    let chars: Vec<char> = username.chars().collect();
    if chars.len() <= 2 {
        "***".to_string()
    } else {
        format!("{}***", chars[..2].iter().collect::<String>())
    }
}

/// 用户 is_active 状态内存缓存的 TTL（5 分钟）
///
/// 安全漏洞 #6 修复：禁用 / 软删除用户的旧 JWT 在剩余有效期（最长 2 小时）内
/// 仍可使用。引入 5 分钟缓存目的是在 DB 压力可接受的前提下，最坏延迟 5 分钟
/// 旧 JWT 失效。TTL 不可过长，否则对管理员封号操作的感知不灵敏；
/// 不可过短，否则失去缓存价值。
const USER_ACTIVE_CACHE_TTL_SECS: u64 = 300;

/// 全局进程级用户 is_active 状态缓存
///
/// key = user_id，value = (is_active, 写入时间戳)
/// 一旦 JWT 通过签名验证，命中本地缓存即视为活跃；
/// 缓存 miss 或 TTL 过期时回查 DB。
static USER_ACTIVE_CACHE: OnceLock<DashMap<i32, (bool, Instant)>> = OnceLock::new();

/// 获取（或惰性初始化）全局用户活跃状态缓存
fn user_active_cache() -> &'static DashMap<i32, (bool, Instant)> {
    USER_ACTIVE_CACHE.get_or_init(DashMap::new)
}

/// 检查用户是否处于活跃状态（5 分钟内存缓存）
///
/// 安全漏洞 #6 修复核心：用于在 `auth_middleware` 中快速校验 JWT 持有者的
/// `is_active` 状态，避免每次请求都查 DB。命中缓存时为一次 DashMap 查，
/// 未命中时为一次 DB 查 + 一次 DashMap 写。
///
/// # 返回
/// - `true`：用户在最近 5 分钟内被确认为 `is_active = true`
/// - `false`：用户已禁用 / 软删除 / 不存在
///
/// # 注意
/// - 进程内缓存不跨实例同步；多副本部署时部分实例可能短暂持有旧值（最多 5 分钟）
/// - `UserService::delete_user` 已失效 Redis 缓存但**未**失效此本地缓存；
///   这是有意的：5 分钟窗口可接受且可避免在删除路径上加额外清理逻辑
async fn is_user_active_cached(state: &AppState, user_id: i32) -> bool {
    let cache = user_active_cache();

    // 1) 查缓存：5 分钟 TTL 内直接返回
    if let Some(entry) = cache.get(&user_id) {
        let (active, ts) = entry.value();
        if ts.elapsed().as_secs() < USER_ACTIVE_CACHE_TTL_SECS {
            return *active;
        }
    }

    // 2) 缓存 miss / TTL 过期：回查 DB
    let user_service = crate::services::user_service::UserService::new(state.db.clone());
    let active = match user_service.find_by_id(user_id).await {
        Ok(user) => user.is_active,
        // 用户不存在视为非活跃（拒绝该 JWT，fail-secure）
        Err(_) => false,
    };

    // 3) 写缓存
    cache.insert(user_id, (active, Instant::now()));
    active
}

/// 判断是否启用 is_active 实时校验（环境变量开关）
///
/// 默认开启（`true`），通过 `AUTH_CHECK_USER_ACTIVE=false` 可关闭以兼容
/// 性能敏感或历史无 `is_active` 字段的环境。
///
/// L-36 修复（批次 370 v13 复审）：使用 LazyLock 确保首次调用时打印当前值，
/// 消除 silent default（原实现环境变量未设置时静默使用 "true"，无任何日志）。
static USER_ACTIVE_CHECK_ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
    let raw = std::env::var("AUTH_CHECK_USER_ACTIVE").unwrap_or_else(|_| "true".to_string());
    let enabled = raw == "true";
    if std::env::var("AUTH_CHECK_USER_ACTIVE").is_err() {
        tracing::info!(
            "AUTH_CHECK_USER_ACTIVE 未设置，使用默认值 true（实时校验用户活跃状态）"
        );
    } else {
        tracing::info!(
            value = %raw,
            enabled,
            "AUTH_CHECK_USER_ACTIVE 已设置"
        );
    }
    enabled
});

fn is_user_active_check_enabled() -> bool {
    *USER_ACTIVE_CHECK_ENABLED
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    // P2-12c 修复（批次 83 v1 复审）：复用 audit_context 公开的 extract_client_ip helper
    // 原实现优先级错误（X-Forwarded-For → X-Real-IP），且不 split/trim X-Forwarded-For
    // 统一优先级：X-Real-IP → X-Forwarded-For(first, trim) → ConnectInfo → "unknown"
    let client_ip = crate::middleware::audit_context::extract_client_ip(&request);

    // 公共路径跳过认证
    let is_public = is_public_path(&path);
    request
        .extensions_mut()
        .insert(PublicPathCache::new(is_public));
    if is_public {
        info!(path = %path, method = %method, client_ip = %client_ip, "公共路径，跳过认证");
        return Ok(next.run(request).await);
    }

    // 优先从 HttpOnly Cookie 中提取 access_token，兼容旧版 jwt Cookie 与 Authorization Header
    let key = Key::derive_from(state.cookie_secret.as_bytes());
    let cookie_jar = PrivateCookieJar::from_headers(request.headers(), key);
    // 1) 新版命名：access_token（httpOnly）
    let token_from_access_cookie = cookie_jar.get("access_token").map(|c| c.value().to_string());
    // 2) 旧版命名：jwt（httpOnly，向后兼容）
    let token_from_legacy_cookie = cookie_jar.get("jwt").map(|c| c.value().to_string());

    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let has_access_cookie = token_from_access_cookie.is_some();
    let has_legacy_cookie = token_from_legacy_cookie.is_some();
    let has_auth_header = auth_header.is_some();

    let token = if let Some(access_token) = token_from_access_cookie {
        info!(
            path = %path,
            method = %method,
            client_ip = %client_ip,
            "从 access_token Cookie 获取Token"
        );
        access_token
    } else if let Some(legacy_token) = token_from_legacy_cookie {
        info!(
            path = %path,
            method = %method,
            client_ip = %client_ip,
            "从 jwt Cookie (旧版) 获取Token"
        );
        legacy_token
    } else if let Some(header_val) = auth_header {
        if !header_val.starts_with("Bearer ") {
            // 低危 #4 修复：避免完整 Authorization 头值落地到日志聚合系统
            //   仅输出脱敏后的前缀和长度，原始 token 不会进入日志
            warn!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                auth_header = %mask_auth_header(&header_val),
                "无效的认证头格式"
            );
            return Err(unauthorized_response("无效的认证头格式"));
        }
        info!(
            path = %path,
            method = %method,
            client_ip = %client_ip,
            "从Authorization头获取Token"
        );
        header_val[7..].to_string()
    } else {
        warn!(
            path = %path,
            method = %method,
            client_ip = %client_ip,
            "缺少认证凭据 (Cookie={}/{}/Header={})",
            has_access_cookie,
            has_legacy_cookie,
            has_auth_header
        );
        return Err(unauthorized_response("缺少认证凭据"));
    };

    if token.is_empty() {
        warn!(path = %path, method = %method, client_ip = %client_ip, "认证失败: 令牌为空");
        return Err(unauthorized_response("认证令牌为空"));
    }

    // 检查 Token 是否在黑名单中
    let is_blacklisted = state.cache.get_token_blacklist().get(&token).is_some();
    if is_blacklisted {
        warn!(path = %path, method = %method, client_ip = %client_ip, "认证失败: Token已被吊销");
        return Err(unauthorized_response("令牌已被吊销，请重新登录"));
    }

    let mut claims = AuthService::validate_token_static(&token, &state.jwt_secret);

    // API 密钥轮换机制：如果当前密钥验证失败，且配置了 previous_jwt_secret，尝试使用旧密钥验证
    if claims.is_err() {
        warn!(path = %path, method = %method, client_ip = %client_ip, "JWT验证失败，尝试使用旧密钥进行平滑过渡");
        if let Some(prev_secret) = &state.previous_jwt_secret {
            claims = AuthService::validate_token_static(&token, prev_secret);
        }
    }

    match claims {
        Ok(claims) => {
            // 检查 JTI 黑名单（已吊销的 session_id 立即拒绝）
            let is_revoked =
                crate::services::auth_service::is_jti_revoked(&claims.session_id).await;
            if is_revoked {
                warn!(
                    path = %path,
                    method = %method,
                    client_ip = %client_ip,
                    jti = %claims.session_id,
                    "认证失败: JTI 已被吊销"
                );
                return Err(unauthorized_response("令牌已被吊销，请重新登录"));
            }

            // 安全漏洞 #9 修复：检查用户级 Token 吊销表
            //    软删除/封禁用户时调用 `revoke_user_jtis(user_id, reason)` 标记该用户，
            //    后续该用户的所有 iat < revoked_at 的 Token 一律拒绝。
            //    与 #6 的 `is_user_active_cached` 互补：#9 是即时进程内黑名单（不依赖 DB/缓存 TTL），
            //    适用于"删除账户后立刻吊销所有活跃 session"的强一致场景。
            let is_user_revoked = crate::services::auth_service::is_user_token_revoked(
                claims.sub,
                claims.iat.timestamp(),
            )
            .await;
            if is_user_revoked {
                let audit_ctx = request.extensions().get::<AuditContext>().cloned();
                warn!(
                    path = %path,
                    method = %method,
                    client_ip = %client_ip,
                    user_id = claims.sub,
                    username = %claims.username,
                    "认证失败: 用户 Token 已被吊销（用户被删除/封禁）"
                );
                audit::log_security_event(
                    SecurityEvent::AuthorizationDenied,
                    claims.sub,
                    &claims.username,
                    claims.role_id,
                    Some("auth_middleware_user_token_revoked"),
                    Some("用户级 Token 已被吊销"),
                    audit_ctx.as_ref(),
                )
                .await;
                return Err(unauthorized_response(
                    "用户已被禁用或删除，请联系管理员",
                ));
            }

            // 安全漏洞 #6 修复：检查用户 is_active 状态
            //    防止被软删除 / 禁用用户的旧 JWT 在剩余有效期（最长 2 小时）内继续使用。
            //    通过 5 分钟本地缓存避免每请求都查 DB；通过环境变量 AUTH_CHECK_USER_ACTIVE
            //    控制开关（默认 true）。
            if is_user_active_check_enabled() && !is_user_active_cached(&state, claims.sub).await {
                // 提取 audit_ctx 供审计日志使用（fail-open：缺省时记 "unknown"）
                let audit_ctx = request.extensions().get::<AuditContext>().cloned();
                warn!(
                    path = %path,
                    method = %method,
                    client_ip = %client_ip,
                    user_id = claims.sub,
                    username = %mask_username(&claims.username),
                    "认证失败: 用户账户已被禁用"
                );
                // best-effort 审计落库（失败不阻塞主流程）
                audit::log_security_event(
                    SecurityEvent::AuthorizationDenied,
                    claims.sub,
                    &claims.username,
                    claims.role_id,
                    Some("auth_middleware_is_active_check"),
                    Some("账户已被禁用"),
                    audit_ctx.as_ref(),
                )
                .await;
                return Err(unauthorized_response("账户已被禁用，请联系管理员"));
            }

            let auth_context = AuthContext::from_claims(claims);
            info!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                user_id = %auth_context.user_id,
                username = %auth_context.username,
                "认证成功"
            );
            request.extensions_mut().insert(auth_context);
            Ok(next.run(request).await)
        }
        Err(e) => {
            warn!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                error = %e,
                "认证失败: 令牌验证失败"
            );
            Err(unauthorized_response("无效的认证令牌"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 低危 #4 修复：测试 Authorization 头脱敏（正常长度）
    #[test]
    fn test_mask_auth_header_normal_length() {
        let token = "Bearer abcdef1234567890.thisisafakesignaturevalue";
        let masked = mask_auth_header(token);
        // 保留前 12 字符
        assert!(masked.starts_with("Bearer abcd"), "应保留前 12 字符前缀");
        // 含长度信息
        assert!(masked.contains("(len="), "应包含原始长度信息");
        // 完整 token 不在脱敏结果中
        assert!(
            !masked.contains("thisisafakesignaturevalue"),
            "完整 token 不应出现在脱敏结果中"
        );
    }

    /// 低危 #4 修复：测试 Authorization 头脱敏（短 header）
    #[test]
    fn test_mask_auth_header_short() {
        let short = "abc";
        let masked = mask_auth_header(short);
        assert_eq!(masked, "***redacted***(len=3)");
    }

    /// 低危 #4 修复：测试 Authorization 头脱敏（边界 = 12 字符）
    #[test]
    fn test_mask_auth_header_boundary() {
        // "Bearer xxxxx" = 12 字符（B-e-a-r-e-r- -x-x-x-x-x），正好等于 PREFIX_KEEP
        // 走 if 分支，不暴露任何前缀字符
        let boundary = "Bearer xxxxx";
        let masked = mask_auth_header(boundary);
        assert_eq!(masked, "***redacted***(len=12)");
    }

    /// 低危 #4 修复：测试用户名脱敏（长用户名）
    #[test]
    fn test_mask_username_long() {
        let masked = mask_username("admin_user");
        assert_eq!(masked, "ad***", "长用户名应保留前 2 字符 + ***");
    }

    /// 低危 #4 修复：测试用户名脱敏（短用户名）
    #[test]
    fn test_mask_username_short() {
        assert_eq!(mask_username("ab"), "***");
        assert_eq!(mask_username("a"), "***");
        assert_eq!(mask_username(""), "***");
    }

    /// 低危 #4 修复：测试中文用户名脱敏（按字符而非字节截断）
    #[test]
    fn test_mask_username_chinese() {
        // 中文字符 1 个 = 3 字节，chars() 按 Unicode 字符截断
        // "管理员" = 3 字符 > 2 字符阈值，走 else 分支保留前 2 字符 → "管理***"
        // 关键：不能按字节截断（`&username[..6]` 在中文上会 panic at boundary）
        let masked = mask_username("管理员");
        assert_eq!(masked, "管理***", "中文用户名应按字符截断，保留前 2 字符");
    }
}
