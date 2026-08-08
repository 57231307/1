//! 中间件配置（CORS / 安全头 / 中间件链）
//!
//! 职责：构建 CORS 层、为完整模式和 Setup 模式分别组装中间件链、
//! 条件注入 HSTS 头。所有安全头和中间件执行顺序均与原 main.rs 保持一致。

use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, Request};
use axum::Router;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Span};

use crate::container::AppState;
use crate::middleware::auth::auth_middleware;
use crate::middleware::circuit_breaker::circuit_breaker_middleware;
use crate::middleware::csrf::csrf_middleware;
use crate::middleware::permission::permission_middleware;
use crate::middleware::rate_limit::rate_limit_by_ip;
use crate::middleware::request_validator::request_logging_middleware;
use crate::routes::create_router;

// ============================================================================
// 安全漏洞 #8 修复：HTTP 请求体大小限制常量
// ============================================================================
// 12MB 全局请求体上限（CSV 导入 10MB + 2MB JSON 编码/头部余量）
/// 全局 HTTP 请求体大小上限：12 MB
pub const MAX_HTTP_BODY_BYTES: usize = 12 * 1024 * 1024;

/// 构建 CORS 中间件层，基于配置白名单动态校验 Origin。
pub fn build_cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            move |origin: &HeaderValue, _request_parts: &axum::http::request::Parts| {
                // 动态验证 Origin 是否在白名单中
                let origin_str = origin.to_str().unwrap_or("");

                // 拒绝通配符，仅允许精确匹配
                allowed_origins.iter().any(|allowed| allowed == origin_str)
            },
        ))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            // P2 2-6 修复：补齐 PATCH 方法，支持 PATCH 部分更新场景的 CORS 预检通过
            Method::PATCH,
            Method::DELETE,
            // batch-10/12 P3：补齐 HEAD 方法，兼容监控探针
            Method::HEAD,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true) // 因为改成了 Cookie 鉴权，必须设置为 true
        .max_age(Duration::from_secs(86400)) // 24小时
}

/// 为完整模式路由应用全部中间件链（timeout→security→rate_limit→auth→...→body_limit）。
pub fn apply_full_mode_layers(app_state: AppState, cors: CorsLayer) -> Router {
    let s_auth = app_state.clone();
    let s_permission = app_state.clone();
    let s_request_validator = app_state.clone();
    let s_metrics = app_state.clone();
    let s_csrf = app_state.clone();
    let s_omni_audit = app_state.clone();
    let s_rate_limit = app_state.clone();
    let s_dynamic_router = app_state.clone();

    let router = create_router(app_state);
    let router = apply_body_limit_and_context(router);
    let router = apply_metrics_layer(router, s_metrics);
    let router = apply_http_trace_layer(router);
    let router = router.layer(cors);
    let router = apply_auth_chain(
        router,
        s_request_validator,
        s_permission,
        s_csrf,
        s_omni_audit,
        s_auth,
    );
    // V15 P1 20.6-B：API 网关熔断中间件（5s 窗口失败率 > 50% 触发 open，30s 后 half-open 探测）
    // 放在 auth_chain 之外、rate_limiting 之内：监控认证后的业务处理 5xx 失败率
    let router = router.layer(axum::middleware::from_fn(circuit_breaker_middleware));
    // V15 P2 20.6-A：API 网关动态路由中间件（根据 api_endpoints 表状态动态放行/拒绝）
    let router = router.layer(axum::middleware::from_fn_with_state(
        s_dynamic_router,
        crate::middleware::dynamic_router::dynamic_router_middleware,
    ));
    let router = apply_rate_limiting(router, s_rate_limit);
    let router = apply_security_headers(router);
    router.layer(axum::middleware::from_fn(
        crate::middleware::timeout::timeout_middleware,
    ))
}

/// 应用 body_limit + audit_context + trace_context（最内层，先注册）。
fn apply_body_limit_and_context(router: Router) -> Router {
    // 安全漏洞 #8 修复：全局 HTTP 请求体大小限制（12MB），防止 OOM DoS。
    // 必须在 cors/trace/metrics 等 layer 之内（先注册），在解析之前拒绝超限请求。
    router
        .layer(DefaultBodyLimit::max(MAX_HTTP_BODY_BYTES))
        // P3.2：审计上下文（在 trace_context 之内层挂载，请求先经 trace_context 注入 trace_id）
        .layer(axum::middleware::from_fn(
            crate::middleware::audit_context::audit_context_middleware,
        ))
        // P3.3：分布式追踪上下文（最外层，确保下游都能拿到 trace_id）
        .layer(axum::middleware::from_fn(
            crate::middleware::trace_context::trace_context_middleware,
        ))
}

/// 应用 Prometheus 指标中间件（记录 method/route/status/耗时）。
fn apply_metrics_layer(router: Router, s_metrics: AppState) -> Router {
    // P3.2：Prometheus 指标中间件（外层，记录所有请求的 method/route/status/耗时）
    router.layer(axum::middleware::from_fn_with_state(
        s_metrics,
        crate::middleware::metrics::metrics_middleware,
    ))
}

/// 提取 HTTP 请求头值（缺失时返回默认值）。
fn extract_header_value(headers: &axum::http::HeaderMap, name: &str, default: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(default)
        .to_string()
}

/// 应用 HTTP TraceLayer（请求开始/完成/失败日志）。
fn apply_http_trace_layer(router: Router) -> Router {
    router.layer(
        TraceLayer::new_for_http()
            .on_request(|request: &Request<_>, _span: &Span| {
                let client_ip = crate::middleware::audit_context::extract_client_ip(request);
                let headers = request.headers();
                let user_agent = extract_header_value(headers, "user-agent", "unknown");
                let origin = extract_header_value(headers, "origin", "none");
                info!(
                    method = %request.method(),
                    uri = %request.uri(),
                    client_ip = %client_ip,
                    user_agent = %user_agent,
                    origin = %origin,
                    "开始处理请求"
                );
            })
            .on_response(
                |response: &axum::response::Response, latency: Duration, _span: &Span| {
                    let status = response.status();
                    if status.is_success() {
                        info!(status = %status, latency_ms = %latency.as_millis(), "请求完成");
                    } else {
                        warn!(status = %status, latency_ms = %latency.as_millis(), "请求异常");
                    }
                },
            )
            .on_failure(
                |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                    warn!("请求失败：{:?} (耗时: {}ms)", error, latency.as_millis());
                },
            ),
    )
}

/// 应用认证链：request_validator → permission → csrf → omni_audit → auth。
fn apply_auth_chain(
    router: Router,
    s_request_validator: AppState,
    s_permission: AppState,
    s_csrf: AppState,
    s_omni_audit: AppState,
    s_auth: AppState,
) -> Router {
    // 中间件执行顺序：auth（最外层、先执行）→ omni_audit → csrf → permission → request_logging → handler
    router
        .layer(axum::middleware::from_fn_with_state(
            s_request_validator,
            request_logging_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            s_permission,
            permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(s_csrf, csrf_middleware))
        // P0 8-1 修复：omni_audit_middleware 全局挂载
        // 在 auth 之后执行（可读 AuthContext），在 csrf/permission 之前（即使被拦截也留审计日志）
        .layer(axum::middleware::from_fn_with_state(
            s_omni_audit,
            crate::middleware::omni_audit::omni_audit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(s_auth, auth_middleware))
}

/// 应用全局 IP 限流（180 req/min，最外层，对所有请求生效）。
fn apply_rate_limiting(router: Router, s_rate_limit: AppState) -> Router {
    // P1 7-4 修复：全局挂载 rate_limit_by_ip，防止匿名 DoS。
    // 挂载在最外层（auth 之外），对未认证请求也生效。
    router.layer(axum::middleware::from_fn_with_state(
        s_rate_limit,
        rate_limit_by_ip,
    ))
}

/// 应用安全响应头：X-Content-Type / X-Frame / X-XSS / CSP / Referrer / Permissions。
fn apply_security_headers(router: Router) -> Router {
    router
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        ))
        // 批次 97 P1-14 修复：csp_middleware 提供"仅在响应头未设置 CSP 时注入"语义
        .layer(axum::middleware::from_fn(
            crate::middleware::csp::csp_middleware,
        ))
        // P3 7-14 修复：HSTS 头移到 match 后条件注入，仅 production 环境生效
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        ))
}

/// 为 Setup 模式应用基础中间件链（TraceLayer + CORS + 安全头）。
/// Setup 模式仅暴露 /init/* 接口，无需认证/权限/CSRF 等业务中间件。
pub fn apply_init_mode_layers(router: Router, cors: CorsLayer) -> Router {
    router
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &Request<_>, _span: &Span| {
                    info!(
                        method = %request.method(),
                        uri = %request.uri(),
                        "开始处理请求"
                    );
                })
                .on_response(
                    |response: &axum::response::Response, latency: Duration, _span: &Span| {
                        info!(
                            status = %response.status(),
                            latency_ms = %latency.as_millis(),
                            "请求完成"
                        );
                    },
                ),
        )
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CONTENT_SECURITY_POLICY,
            // V15 P2 B03-P2-7：与 csp.rs CSP_POLICY 对齐，移除 script-src 的 unsafe-inline 和 wasm-unsafe-eval
            HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests;"),
        ))
        // P3 7-14 修复：HSTS 头移到 match 后条件注入，仅 production 环境生效
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        ))
}

/// P3 7-14 修复：HSTS 头仅在 production 环境注入。
/// 原实现无条件注入，但 HTTP 模式下浏览器会忽略，开发环境无效。
pub fn apply_hsts_if_production(app: Router) -> Router {
    if crate::utils::config::is_production() {
        app.layer(SetResponseHeaderLayer::overriding(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
        ))
    } else {
        app
    }
}
