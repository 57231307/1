//! 初始化接口 Token 校验中间件（bug.md #3 修复）
//!
//! 防御 init 子系统的"窗口期攻击"：在系统首次部署（数据库无 users 表）时，
//! `/api/v1/erp/init/initialize` 等接口因 `PUBLIC_PATHS` 包含 `/api/v1/erp/init`
//! 前缀而被 `auth_middleware` 短路跳过 JWT 验证。攻击者可抢先初始化。
//!
//! 修复方案：
//! 1. 部署时在环境变量 `INIT_TOKEN` 配置一个长随机字符串
//! 2. 初始化请求必须携带 `X-Init-Token: <token>` 请求头
//! 3. 使用 `subtle::ConstantTimeEq` 防时序攻击
//! 4. 缺失/错误 Token 直接返回 401 Unauthorized
//!
//! 注意：仅对"高危"初始化接口（initialize、initialize-with-db、async 变体）生效；
//! `get_init_status`、`test_database_connection`、`get_task_status` 等只读/受限接口
//! 不在此中间件覆盖范围（前者只返回初始化状态，后者已有 admin 二次校验）。

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

/// 初始化 Token 请求头名称
pub const INIT_TOKEN_HEADER: &str = "X-Init-Token";

/// 初始化 Token 环境变量名
pub const INIT_TOKEN_ENV: &str = "INIT_TOKEN";

/// 中间件函数：校验 init Token
///
/// 行为：
/// - 未设置 `INIT_TOKEN` 环境变量 → 401（fail-secure，避免降级到无认证）
/// - 请求头缺失或类型错误 → 401
/// - Token 不匹配 → 401（使用恒定时间比较防时序攻击）
/// - Token 匹配 → 放行
pub async fn init_token_middleware(req: Request<Body>, next: Next) -> Response {
    let init_token = match std::env::var(INIT_TOKEN_ENV) {
        Ok(t) if !t.is_empty() => t,
        _ => {
            tracing::error!(
                "{} 环境变量未配置，初始化接口拒绝所有请求（fail-secure）",
                INIT_TOKEN_ENV
            );
            return init_token_unauthorized(format!(
                "服务器未配置 {}，初始化接口不可用",
                INIT_TOKEN_ENV
            ));
        }
    };

    let provided_token = match extract_token_from_headers(req.headers()) {
        Some(t) => t,
        None => {
            return init_token_unauthorized(format!("缺少 {} 请求头", INIT_TOKEN_HEADER));
        }
    };

    // 使用恒定时间比较，防止时序攻击
    let provided_bytes = provided_token.as_bytes();
    let expected_bytes = init_token.as_bytes();
    if provided_bytes.ct_eq(expected_bytes).unwrap_u8() != 1 {
        tracing::warn!(
            "init 端点收到错误的 {} 头（疑似未授权的初始化尝试）",
            INIT_TOKEN_HEADER
        );
        return init_token_unauthorized(format!("{} 无效", INIT_TOKEN_HEADER));
    }

    next.run(req).await
}

/// 从请求头中提取 init token
fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(INIT_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// 构造 401 响应
fn init_token_unauthorized(reason: String) -> Response {
    let body = serde_json::json!({
        "code": 40101,
        "message": "初始化接口鉴权失败",
        "detail": reason,
    });
    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    /// 创建一个最小化测试 Router
    fn build_test_app() -> Router {
        async fn handler() -> &'static str {
            "ok"
        }
        Router::new()
            .route("/test", get(handler))
            .layer(middleware::from_fn(init_token_middleware))
    }

    /// 场景 A：未设置 INIT_TOKEN 环境变量 → 期望 401
    #[tokio::test]
    async fn test_init_token_missing_env() {
        // 确保环境变量未设置
        std::env::remove_var(INIT_TOKEN_ENV);

        let app = build_test_app();
        let req = Request::builder()
            .uri("/test")
            .header(INIT_TOKEN_HEADER, "any-token")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// 场景 B：未提供 X-Init-Token 头 → 期望 401
    #[tokio::test]
    async fn test_init_token_missing_header() {
        std::env::set_var(INIT_TOKEN_ENV, "test-secret-token");

        let app = build_test_app();
        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        std::env::remove_var(INIT_TOKEN_ENV);
    }

    /// 场景 C：提供错误的 X-Init-Token → 期望 401
    #[tokio::test]
    async fn test_init_token_wrong() {
        std::env::set_var(INIT_TOKEN_ENV, "correct-token");

        let app = build_test_app();
        let req = Request::builder()
            .uri("/test")
            .header(INIT_TOKEN_HEADER, "wrong-token")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        std::env::remove_var(INIT_TOKEN_ENV);
    }

    /// 场景 D：提供正确的 X-Init-Token → 期望 200
    #[tokio::test]
    async fn test_init_token_correct() {
        std::env::set_var(INIT_TOKEN_ENV, "correct-token-abc");

        let app = build_test_app();
        let req = Request::builder()
            .uri("/test")
            .header(INIT_TOKEN_HEADER, "correct-token-abc")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        std::env::remove_var(INIT_TOKEN_ENV);
    }

    /// 场景 E：INIT_TOKEN 配置为空字符串 → 期望 401（fail-secure）
    #[tokio::test]
    async fn test_init_token_empty_env() {
        std::env::set_var(INIT_TOKEN_ENV, "");

        let app = build_test_app();
        let req = Request::builder()
            .uri("/test")
            .header(INIT_TOKEN_HEADER, "any-token")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        std::env::remove_var(INIT_TOKEN_ENV);
    }
}
