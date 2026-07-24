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

use crate::middleware::auth::auth_middleware;
use crate::middleware::csrf::csrf_middleware;
use crate::middleware::permission::permission_middleware;
use crate::middleware::rate_limit::rate_limit_by_ip;
use crate::middleware::request_validator::request_validator_middleware;
use crate::routes::create_router;
use crate::utils::app_state::AppState;

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

/// 为完整模式（数据库已连接）路由应用全部中间件链。
///
/// 中间件执行顺序（从外到内）：
/// timeout → security headers → rate_limit → auth → omni_audit → csrf → permission
/// → request_validator → cors → trace → metrics → trace_context → audit_context
/// → body_limit → handler
pub fn apply_full_mode_layers(app_state: AppState, cors: CorsLayer) -> Router {
    // 克隆 AppState 供各 from_fn_with_state 中间件使用，原始 app_state 移入 create_router
    let s_auth = app_state.clone();
    let s_permission = app_state.clone();
    let s_request_validator = app_state.clone();
    let s_metrics = app_state.clone();
    let s_csrf = app_state.clone();
    let s_omni_audit = app_state.clone();
    let s_rate_limit = app_state.clone();

    create_router(app_state)
        // 安全漏洞 #8 修复：全局 HTTP 请求体大小限制（12MB）
        // - 设计目的：兜底防止已认证用户发送 100MB+ 请求触发 OOM DoS
        // - 上限选择：CSV 导入允许 10MB（业务上限），12MB 留 2MB 余量
        //   给 JSON 编码 / 头部开销，避免 10MB CSV + metadata 触及外层限制
        // - 与 DTO 校验 + handler 早期校验 + service defense-in-depth 形成
        //   四层防御，详见 import_export_service.rs 模块顶部注释
        // - 必须在 cors / trace / metrics 等其他 layer 之外（先执行），
        //   这样在所有解析之前先拒绝超限请求，节省后续中间件开销
        //
        // 使用具名常量 `MAX_HTTP_BODY_BYTES`（=12MB）替代魔法数字，
        // 防御 clippy 1.94 对 `12 * 1024 * 1024` 字面量的 dead_code 误报。
        .layer(DefaultBodyLimit::max(MAX_HTTP_BODY_BYTES))
        // P3.2：审计上下文（必须在 trace_context 之内层挂载，
        // 即在 .layer() 链中位于 trace_context 之前；这样请求先经过
        // trace_context 注入 trace_id，再进入 audit_context 读取并补充 IP/UA）
        .layer(axum::middleware::from_fn(
            crate::middleware::audit_context::audit_context_middleware,
        ))
        // P3.3：分布式追踪上下文（最最外层，确保下游都能拿到 trace_id）
        .layer(axum::middleware::from_fn(
            crate::middleware::trace_context::trace_context_middleware,
        ))
        // P3.2：Prometheus 指标中间件（外层，记录所有请求的 method/route/status/耗时）
        .layer(axum::middleware::from_fn_with_state(
            s_metrics,
            crate::middleware::metrics::metrics_middleware,
        ))
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &Request<_>, _span: &Span| {
                    // P3 维度 12 修复（批次 87）：复用 audit_context::extract_client_ip helper，
                    // 消除 IP 提取逻辑重复（原批次 86 内联实现 30 行 → 单行 helper 调用）
                    let client_ip =
                        crate::middleware::audit_context::extract_client_ip(request);
                    let user_agent = request
                        .headers()
                        .get("user-agent")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown")
                        .to_string();
                    let origin = request
                        .headers()
                        .get("origin")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("none")
                        .to_string();
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
                            info!(
                                status = %status,
                                latency_ms = %latency.as_millis(),
                                "请求完成"
                            );
                        } else {
                            warn!(
                                status = %status,
                                latency_ms = %latency.as_millis(),
                                "请求异常"
                            );
                        }
                    },
                )
                .on_failure(
                    |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                        warn!("请求失败：{:?} (耗时: {}ms)", error, latency.as_millis());
                    },
                ),
        )
        .layer(cors)
        // 中间件执行顺序：auth_middleware（最后注册、最外层、先执行）→ csrf_middleware → permission_middleware → request_validator → 处理器
        .layer(axum::middleware::from_fn_with_state(s_request_validator, request_validator_middleware))
        .layer(axum::middleware::from_fn_with_state(s_permission, permission_middleware))
        .layer(axum::middleware::from_fn_with_state(s_csrf, csrf_middleware))
        // P0 8-1 修复：omni_audit_middleware 全局挂载（取代各业务路由局部挂载）
        // 执行顺序：auth → omni_audit → csrf → permission → request_validator → handler
        // 设计要点：
        // 1. 在 auth_middleware 之后执行，可读取 AuthContext（已认证用户信息）
        // 2. 在 csrf/permission 之前执行，确保即使被 csrf/permission 拦截也能留下审计日志
        // 3. omni_audit.rs 内部已跳过 PUBLIC_PATHS（登录/刷新含密码）及 metrics/health/swagger/static
        // 4. 配合 P0 8-5（audit_log/omni_audit 查询接口 admin 校验），形成完整审计闭环
        .layer(axum::middleware::from_fn_with_state(
            s_omni_audit,
            crate::middleware::omni_audit::omni_audit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(s_auth, auth_middleware))
        // P1 7-4 修复：全局挂载 rate_limit_by_ip（180 req/min）
        // 修复背景：rate_limit_by_ip 中间件已实现但未在 main.rs 全局挂载，
        // 所有业务 API 无限流，可被 DoS。
        // 修复方案：挂载在最外层（auth_middleware 之外），对所有请求生效，
        // 包括未认证请求，防止匿名 DoS。
        // 执行顺序（从外到内）：rate_limit → auth → omni_audit → csrf → permission → request_validator → handler
        .layer(axum::middleware::from_fn_with_state(
            s_rate_limit,
            rate_limit_by_ip,
        ))
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
        // 批次 97 P1-14 修复（v5 复审）：将 csp_middleware 真实挂载到全局路由，
        // 替代原 SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, ...)。
        // csp_middleware 提供"仅在响应头尚未设置 CSP 时注入"语义，
        // 支持路由级精细化覆盖（路由可自定义 CSP 头，中间件不覆盖）。
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
        .layer(axum::middleware::from_fn(crate::middleware::timeout::timeout_middleware))
}

/// 为 Setup 模式（数据库未连接）路由应用基础中间件链。
///
/// Setup 模式仅暴露 /init/* 系列接口，不需要认证/权限/CSRF 等业务中间件，
/// 仅保留 TraceLayer + CORS + 安全头。
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
            HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none';"),
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
///
/// 原实现无条件注入，但 HTTP 模式下浏览器会忽略 HSTS 头，开发环境无效。
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
