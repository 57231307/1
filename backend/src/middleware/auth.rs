use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::services::auth_service::AuthService;
use crate::utils::app_state::AppState;
use crate::utils::cache::Cache;
use crate::utils::request_ext::PublicPathCache;
use crate::utils::response::unauthorized_response;
use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use axum_extra::extract::cookie::{Key, PrivateCookieJar};
use tracing::{info, warn};

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

    // 优先从 HttpOnly Cookie 中提取 jwt，兼容 Authorization Header
    let key = Key::derive_from(state.cookie_secret.as_bytes());
    let cookie_jar = PrivateCookieJar::from_headers(request.headers(), key);
    let token_from_cookie = cookie_jar.get("jwt").map(|c| c.value().to_string());

    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let has_cookie = token_from_cookie.is_some();
    let has_auth_header = auth_header.is_some();

    let token = if let Some(cookie_token) = token_from_cookie {
        info!(path = %path, method = %method, client_ip = %client_ip, "从Cookie获取Token");
        cookie_token
    } else if let Some(header_val) = auth_header {
        if !header_val.starts_with("Bearer ") {
            warn!(path = %path, method = %method, client_ip = %client_ip, "无效的认证头格式: {}", header_val);
            return Err(unauthorized_response("无效的认证头格式"));
        }
        info!(path = %path, method = %method, client_ip = %client_ip, "从Authorization头获取Token");
        header_val[7..].to_string()
    } else {
        warn!(path = %path, method = %method, client_ip = %client_ip, "缺少认证凭据 (Cookie={}, Header={})", has_cookie, has_auth_header);
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
                warn!(path = %path, method = %method, client_ip = %client_ip, jti = %claims.session_id, "认证失败: JTI 已被吊销");
                return Err(unauthorized_response("令牌已被吊销，请重新登录"));
            }

            let auth_context = AuthContext::from_claims(claims);
            info!(path = %path, method = %method, client_ip = %client_ip, user_id = %auth_context.user_id, username = %auth_context.username, "认证成功");
            request.extensions_mut().insert(auth_context);
            Ok(next.run(request).await)
        }
        Err(e) => {
            warn!(path = %path, method = %method, client_ip = %client_ip, error = %e, "认证失败: 令牌验证失败");
            Err(unauthorized_response("无效的认证令牌"))
        }
    }
}
