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
fn is_user_active_check_enabled() -> bool {
    std::env::var("AUTH_CHECK_USER_ACTIVE").unwrap_or_else(|_| "true".to_string()) == "true"
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

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
            warn!(
                path = %path,
                method = %method,
                client_ip = %client_ip,
                "无效的认证头格式: {}",
                header_val
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
                    username = %claims.username,
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
