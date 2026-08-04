use axum::extract::ConnectInfo;
use axum::{
    body::{to_bytes, Body, Bytes},
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::time::Instant;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::services::sensitive_action_alert::SensitiveActionAlert;

/// 请求元数据：从请求中提取的方法/URI/查询串/常见 Header
struct RequestMeta {
    method: String,
    uri: String,
    query_string: String,
    user_agent: Option<String>,
    referer: Option<String>,
    content_type: Option<String>,
    accept: Option<String>,
    x_forwarded_for: Option<String>,
    x_real_ip: Option<String>,
}

/// 审计上下文：跨 send_audit_log/build_audit_message/build_audit_payload 共享的请求/响应审计数据
/// 引入此结构体避免辅助函数参数列表过长（13/14 参 → 2/3 参），符合 clippy 的 `too many arguments` lint 阈值（默认 7）。
struct AuditContext<'a> {
    trace_id: &'a str,
    meta: &'a RequestMeta,
    user_id: Option<i32>,
    username: &'a str,
    ip_address: &'a Option<String>,
    request_body: &'a Option<String>,
    response_body: &'a str,
    response_content_type: &'a Option<String>,
    status_code: StatusCode,
    status_str: &'a str,
    duration_ms: i32,
    duration_secs: f64,
}

/// 全局审计中间件：记录所有 API 请求的方法/路径/用户/IP/请求体/响应体/耗时
pub async fn omni_audit_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();
    let trace_id = uuid::Uuid::new_v4().to_string();

    let meta = extract_request_meta(&req);
    let (user_id, username) = extract_user_context(&req);
    let ip_address = resolve_client_ip(&meta, &req);

    let (req, request_body) =
        read_request_body_for_audit(req, &meta.method, &meta.uri, &trace_id).await;

    log_request_start(&trace_id, &meta, user_id, &username, &ip_address);

    let response = next.run(req).await;
    let duration_ms = start_time.elapsed().as_millis() as i32;
    let duration_secs = start_time.elapsed().as_secs_f64();

    let (response, response_body, response_content_type) =
        read_response_body(response, &meta.method, &meta.uri, &trace_id).await;
    let status_code = response.status();
    let status_str = classify_status_str(status_code);

    log_request_complete(&trace_id, &meta, status_code, duration_secs, &response_body);

    if !should_skip_audit_path(&meta.uri) {
        let audit_ctx = AuditContext {
            trace_id: &trace_id,
            meta: &meta,
            user_id,
            username: &username,
            ip_address: &ip_address,
            request_body: &request_body,
            response_body: &response_body,
            response_content_type: &response_content_type,
            status_code,
            status_str: &status_str,
            duration_ms,
            duration_secs,
        };
        send_audit_log(&state, &audit_ctx);
    }

    Ok(response)
}

/// 提取请求元数据：method/uri/query_string 及 User-Agent/Referer/Content-Type/Accept/X-Forwarded-For/X-Real-IP
fn extract_request_meta(req: &Request<Body>) -> RequestMeta {
    let method = req.method().to_string();
    let uri = req.uri().path().to_string();
    let query_string = req.uri().query().map(|q| q.to_string()).unwrap_or_default();
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let referer = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let accept = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    // X-Forwarded-For 可能含多个 IP，取首段（最原始客户端 IP）并 trim
    let x_forwarded_for = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string());
    let x_real_ip = req
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    RequestMeta {
        method,
        uri,
        query_string,
        user_agent,
        referer,
        content_type,
        accept,
        x_forwarded_for,
        x_real_ip,
    }
}

/// 解析客户端 IP（优先级：X-Real-IP > X-Forwarded-For > 连接地址）
fn resolve_client_ip(meta: &RequestMeta, req: &Request<Body>) -> Option<String> {
    if meta.x_real_ip.is_some() || meta.x_forwarded_for.is_some() {
        meta.x_real_ip.clone().or(meta.x_forwarded_for.clone())
    } else {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip().to_string())
    }
}

/// 从 AuthContext 提取用户 ID 和用户名（未认证请求 user_id 为 None）
fn extract_user_context(req: &Request<Body>) -> (Option<i32>, String) {
    let user_id = req.extensions().get::<AuthContext>().map(|ctx| ctx.user_id);
    let username = req
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.username.clone())
        .unwrap_or_default();
    (user_id, username)
}

/// 读取请求体（仅 POST/PUT/PATCH）：敏感路径脱敏 + 超 5000 字符截断
async fn read_request_body_for_audit(
    req: Request<Body>,
    method: &str,
    uri: &str,
    trace_id: &str,
) -> (Request<Body>, Option<String>) {
    if method != "POST" && method != "PUT" && method != "PATCH" {
        return (req, None);
    }

    let (parts, body) = req.into_parts();
    // body 读取失败时记录 warn 日志而非静默回退空字节
    let body_bytes = match to_bytes(body, 50 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                "[{}] {} {} 请求体读取失败，审计记录 body 为空: {}",
                trace_id,
                method,
                uri,
                e
            );
            Bytes::new()
        }
    };
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    let req = Request::from_parts(parts, Body::from(body_bytes));

    // 敏感路径（change-password/reset-totp 等）请求体脱敏为 "[REDACTED]"
    let is_sensitive_path = is_sensitive_request_body_path(uri);
    let body_for_audit = if is_sensitive_path {
        "[REDACTED]".to_string()
    } else {
        // V15 P2 B17-P2-21：非敏感路径请求体也做 PII 脱敏（手机号/邮箱/身份证号）
        crate::utils::field_mask::mask_text_pii(&body_str)
    };

    let truncated_body = truncate_text(&body_for_audit, 5000);
    (req, Some(truncated_body))
}

/// 记录请求开始日志
fn log_request_start(
    trace_id: &str,
    meta: &RequestMeta,
    user_id: Option<i32>,
    username: &str,
    ip_address: &Option<String>,
) {
    tracing::info!(
        "[{}] {} {} 开始 | 用户: {}({}) | IP: {} | Query: {} | Content-Type: {}",
        trace_id,
        meta.method,
        meta.uri,
        username,
        user_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "anonymous".to_string()),
        ip_address.as_deref().unwrap_or("unknown"),
        meta.query_string,
        meta.content_type.as_deref().unwrap_or("-")
    );
}

/// 根据 HTTP 状态码分类审计状态（SUCCESS/DENIED/FAILED）
fn classify_status_str(status_code: StatusCode) -> String {
    if status_code.is_success() {
        "SUCCESS".to_string()
    } else if status_code == StatusCode::UNAUTHORIZED || status_code == StatusCode::FORBIDDEN {
        "DENIED".to_string()
    } else {
        "FAILED".to_string()
    }
}

/// 读取响应体（限制 10KB），body 读取失败时记录 warn 日志
async fn read_response_body(
    response: Response,
    method: &str,
    uri: &str,
    trace_id: &str,
) -> (Response, String, Option<String>) {
    let (parts, body) = response.into_parts();
    let body_bytes = match to_bytes(body, 10 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                "[{}] {} {} 响应体读取失败，审计记录 response_body 为空: {}",
                trace_id,
                method,
                uri,
                e
            );
            Bytes::new()
        }
    };
    let response_body = String::from_utf8_lossy(&body_bytes).to_string();
    // V15 P2 B17-P2-21：响应体日志做 PII 脱敏（手机号/邮箱/身份证号）
    let response_body = crate::utils::field_mask::mask_text_pii(&response_body);
    let response_content_type = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let response = Response::from_parts(parts, Body::from(body_bytes));
    (response, response_body, response_content_type)
}

/// 记录请求完成日志（成功用 info，失败用 warn 并截断错误信息）
fn log_request_complete(
    trace_id: &str,
    meta: &RequestMeta,
    status_code: StatusCode,
    duration_secs: f64,
    response_body: &str,
) {
    if status_code.is_success() {
        tracing::info!(
            "[{}] {} {} 完成 | 状态: {} | 耗时: {:.3}s | 响应大小: {} bytes",
            trace_id,
            meta.method,
            meta.uri,
            status_code.as_u16(),
            duration_secs,
            response_body.len()
        );
    } else {
        tracing::warn!(
            "[{}] {} {} 失败 | 状态: {} | 耗时: {:.3}s | 错误: {}",
            trace_id,
            meta.method,
            meta.uri,
            status_code.as_u16(),
            duration_secs,
            truncate_text(response_body, 500)
        );
    }
}

/// 判断路径是否跳过审计（公开路径/metrics/health/swagger/api-docs/static）
fn should_skip_audit_path(uri: &str) -> bool {
    is_public_path(uri)
        || uri.starts_with("/metrics")
        || uri.starts_with("/health")
        || uri.starts_with("/swagger-ui")
        || uri.starts_with("/api-docs")
        || uri.starts_with("/static")
}

/// 按 Unicode 字符截断文本（超 limit 字符追加 "..."，避免字节切片切到多字节字符中间 panic）
fn truncate_text(text: &str, limit: usize) -> String {
    if text.chars().count() > limit {
        format!("{}...", text.chars().take(limit).collect::<String>())
    } else {
        text.to_string()
    }
}

/// 发送审计日志：推断模块/检查敏感操作/构建消息/写入审计队列
fn send_audit_log(state: &AppState, ctx: &AuditContext<'_>) {
    let module = infer_module_from_path(&ctx.meta.uri);

    let _sensitive_action = SensitiveActionAlert::check_and_alert(
        &ctx.meta.method,
        &module,
        ctx.user_id.unwrap_or(0),
        ctx.username,
        ctx.ip_address.as_deref(),
    );

    let resource_id = extract_resource_id(&ctx.meta.uri);
    let message = build_audit_message(ctx, &module, &resource_id);
    state.omni_audit.log(message);
}

/// 构建审计日志消息体（含 payload JSON 和错误信息）
fn build_audit_message(
    ctx: &AuditContext<'_>,
    module: &str,
    resource_id: &Option<String>,
) -> OmniAuditMessage {
    let truncated_response = truncate_text(ctx.response_body, 2000);
    let error_msg = if !ctx.status_code.is_success() {
        Some(truncated_response.clone())
    } else {
        None
    };
    let payload = build_audit_payload(ctx, &truncated_response);
    // V15 P1-7-1：按 method+path+query 分类操作类型（PRINT/EXPORT/DOWNLOAD/READ/CREATE/UPDATE/DELETE）
    let event_type = classify_operation(&ctx.meta.method, &ctx.meta.uri, &ctx.meta.query_string);
    OmniAuditMessage {
        trace_id: ctx.trace_id.to_string(),
        user_id: ctx.user_id,
        username: Some(ctx.username.to_string()),
        event_type,
        event_name: format!("{} {}", ctx.meta.method, ctx.meta.uri),
        resource: ctx.meta.uri.clone(),
        action: ctx.meta.method.clone(),
        resource_type: Some(module.to_string()),
        resource_id: resource_id.clone(),
        resource_name: None,
        description: Some(format!(
            "{} {} - {}",
            ctx.meta.method,
            ctx.meta.uri,
            ctx.status_code.as_u16()
        )),
        payload: Some(payload),
        ip_address: ctx.ip_address.clone(),
        user_agent: ctx.meta.user_agent.clone(),
        request_method: Some(ctx.meta.method.clone()),
        request_path: Some(ctx.meta.uri.clone()),
        request_body: ctx.request_body.clone(),
        duration_ms: ctx.duration_ms,
        status: ctx.status_str.to_string(),
        error_msg,
        old_value: None,
        new_value: None,
        condition: if ctx.meta.query_string.is_empty() {
            None
        } else {
            Some(ctx.meta.query_string.clone())
        },
    }
}

/// V15 P1-7-1：按 method+path+query 分类操作类型；分类规则（优先级从高到低）： 1. 路径末段为 print / 路径含 /print/ → PRINT 2. 路径末段为 export / 路径含 /export/ / 路径以 /pdf 结尾 → EXPORT 3. 查询参数 action=download / 路径末段为 download → DOWNLOAD
/// 4. HTTP 方法映射：GET→READ、POST→CREATE、PUT/PATCH→UPDATE、DELETE→DELETE 5. 其他 → OTHER；用途：omni_audit_logs.event_type 字段从硬编码 "API_CALL" 升级为分类标签， 支持 SQL `WHERE event_type = 'EXPORT'` 筛选导出操作，满足合规审计报表分类需求。
fn classify_operation(method: &str, uri: &str, query_string: &str) -> String {
    // 路径末段（剥离 query string）
    let path = uri.split('?').next().unwrap_or(uri);
    let last_segment = path.split('/').rfind(|p| !p.is_empty()).unwrap_or("");

    // 1. PRINT：路径末段为 print，或路径含 /print/
    if last_segment == "print" || path.contains("/print/") {
        return "PRINT".to_string();
    }

    // 2. EXPORT：路径末段为 export，或路径含 /export/，或路径以 /pdf 结尾
    if last_segment == "export"
        || path.contains("/export/")
        || last_segment == "pdf"
        || path.ends_with("/pdf")
    {
        return "EXPORT".to_string();
    }

    // 3. DOWNLOAD：查询参数 action=download，或路径末段为 download
    if last_segment == "download" {
        return "DOWNLOAD".to_string();
    }
    if !query_string.is_empty() {
        for pair in query_string.split('&') {
            let mut parts = pair.splitn(2, '=');
            if parts.next() == Some("action") {
                if let Some(value) = parts.next() {
                    let decoded = percent_encoding::percent_decode_str(value)
                        .decode_utf8()
                        .unwrap_or_default();
                    if decoded == "download" {
                        return "DOWNLOAD".to_string();
                    }
                }
            }
        }
    }

    // 4. HTTP 方法映射
    match method {
        "GET" => "READ".to_string(),
        "POST" => "CREATE".to_string(),
        "PUT" | "PATCH" => "UPDATE".to_string(),
        "DELETE" => "DELETE".to_string(),
        _ => "OTHER".to_string(),
    }
}

/// 构建审计 payload JSON（含状态码/请求体/响应体/耗时/IP 等）
fn build_audit_payload(ctx: &AuditContext<'_>, truncated_response: &str) -> serde_json::Value {
    serde_json::json!({
        "status_code": ctx.status_code.as_u16(),
        "query_string": ctx.meta.query_string,
        "request_body": ctx.request_body,
        "response_body": truncated_response,
        "response_content_type": ctx.response_content_type,
        "duration_ms": ctx.duration_ms,
        "duration_secs": ctx.duration_secs,
        "ip_address": ctx.ip_address,
        "user_agent": ctx.meta.user_agent,
        "referer": ctx.meta.referer,
        "content_type": ctx.meta.content_type,
        "accept": ctx.meta.accept,
        "response_size_bytes": ctx.response_body.len(),
    })
}

/// 判断请求路径是否为敏感路径（请求体含密码等敏感信息，需脱敏）
fn is_sensitive_request_body_path(uri: &str) -> bool {
    let path = uri.split('?').next().unwrap_or(uri).to_lowercase();
    const SENSITIVE_PATTERNS: &[&str] = &[
        "/auth/change-password",
        "/auth/reset-password",
        "/auth/reset_password",
        "/users/change-password",
        "/users/reset-password",
        "/init/reset-password",
        "/init/reset_password",
        "/setup-totp",
        "/enable-totp",
        "/verify-totp",
        "/totp/setup",
        "/totp/enable",
        "/totp/verify",
        "/totp/disable",
    ];
    SENSITIVE_PATTERNS
        .iter()
        .any(|p| path == *p || path.ends_with(p))
}

/// 根据请求路径推断模块名称
fn infer_module_from_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 {
        parts[3].to_string()
    } else if parts.len() >= 3 {
        parts[2].to_string()
    } else {
        "unknown".to_string()
    }
}

/// 从路径中提取资源ID
fn extract_resource_id(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    parts.last().and_then(|last| {
        if last.chars().all(|c| c.is_numeric()) && !last.is_empty() {
            Some(last.to_string())
        } else {
            None
        }
    })
}
