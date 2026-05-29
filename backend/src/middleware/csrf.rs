use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, Method},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use crate::utils::app_state::AppState;
use serde_json::json;
use tracing::{warn, debug};
use hmac::{Hmac, Mac, KeyInit};
use sha2::Sha256;
use hex;

// 使用 HMAC-SHA256 生成和验证 CSRF Token
type HmacSha256 = Hmac<Sha256>;

/// 生成基于会话的 CSRF Token
fn generate_csrf_token(session_id: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC 密钥长度有效");
    mac.update(session_id.as_bytes());
    mac.update(b"csrf_token_v1");
    let result = mac.finalize();
    let bytes = result.into_bytes();
    hex::encode(&bytes[..16])
}

/// 验证 CSRF Token 是否有效
fn verify_csrf_token(token: &str, session_id: &str, secret: &str) -> bool {
    let expected = generate_csrf_token(session_id, secret);
    // 使用常量时间比较防止时序攻击
    
    if token.len() != expected.len() {
        return false;
    }
    let mut result = 0u8;
    for (a, b) in token.bytes().zip(expected.bytes()) {
        result |= a ^ b;
    }
    result == 0
}

/// 从请求中提取会话标识
/// 优先使用 JWT token 中的 session_id，如果没有则使用 IP + User-Agent 的哈希
fn extract_session_id(request: &Request<Body>, secret: &str) -> Option<String> {
    // 尝试从 Authorization header 提取 JWT
    if let Some(auth_header) = request.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if !token.is_empty() {
                    // 尝试解码 JWT 获取 session_id
                    match crate::services::auth_service::AuthService::validate_token_static(token, secret) {
                        Ok(claims) => return Some(claims.session_id),
                        Err(e) => {
                            // JWT 解码失败，记录警告并回退到其他方法
                            tracing::warn!("Failed to decode JWT for session ID: {}", e);
                        }
                    }
                }
            }
        }
    }

    // 回退到 IP + User-Agent 哈希
    let ip = request.headers()
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .or_else(|| request.headers()
            .get("X-Real-IP")
            .and_then(|h| h.to_str().ok()))
        .unwrap_or("unknown");

    let user_agent = request.headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // 简单组合，不需要加密安全，仅用于区分不同客户端
    Some(format!("client:{}:{}", ip, &user_agent[..user_agent.len().min(50)]))
}

fn forbidden_response(message: &str) -> Response {
    let body = json!({
        "code": 403,
        "message": message,
        "data": null
    });
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}

pub async fn csrf_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method();

    // 只拦截状态变更方法
    if method != Method::POST && method != Method::PUT && method != Method::DELETE && method != Method::PATCH {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();

    // 豁免登录和初始化接口，因为它们还未获得 token
    let public_paths = [
        "/api/v1/erp/auth/login",
        "/api/v1/erp/auth/refresh_token",
        "/api/v1/erp/init",
        "/api/v1/erp/init/test_db",
        "/api/v1/erp/init/initialize",
    ];
    if public_paths.iter().any(|p| path.starts_with(p)) {
        debug!("CSRF 检查跳过公开路径: {}", path);
        return Ok(next.run(request).await);
    }

    // 提取 CSRF Token 从 Header
    let csrf_token_header = request.headers()
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok());

    // 提取 X-Requested-With Header（用于 AJAX 请求识别）
    let x_requested_with = request.headers()
        .get("X-Requested-With")
        .and_then(|h| h.to_str().ok());

    // 策略：
    // 1. 如果提供了 X-CSRF-Token，验证其有效性（双重提交模式）
    // 2. 如果是 AJAX 请求（X-Requested-With: XMLHttpRequest），在现代浏览器中受同源策略保护，允许通过
    // 3. 否则拒绝请求

    if let Some(csrf_token) = csrf_token_header {
        // 验证 CSRF Token 的有效性
        let session_id = extract_session_id(&request, &state.jwt_secret);

        if let Some(session_id) = session_id {
            if verify_csrf_token(csrf_token, &session_id, &state.cookie_secret) {
                debug!("CSRF Token 验证通过: {}", path);
                return Ok(next.run(request).await);
            } else {
                warn!("CSRF Token 验证失败: {} {}", method, path);
                return Err(forbidden_response("CSRF Token 无效"));
            }
        } else {
            warn!("无法提取会话标识，拒绝请求: {} {}", method, path);
            return Err(forbidden_response("无法验证会话"));
        }
    }

    // AJAX 请求在现代浏览器中受同源策略保护，可以作为备选方案
    if x_requested_with == Some("XMLHttpRequest") {
        debug!("AJAX 请求通过 CSRF 检查: {}", path);
        return Ok(next.run(request).await);
    }

    warn!("拒绝可能发生 CSRF 的请求: {} {}。缺少 X-CSRF-Token 或 X-Requested-With 头", method, path);
    Err(forbidden_response("缺少 CSRF Token，请刷新页面重试"))
}

/// 为前端提供获取 CSRF Token 的辅助函数
pub fn create_csrf_token_for_session(session_id: &str, secret: &str) -> String {
    generate_csrf_token(session_id, secret)
}
