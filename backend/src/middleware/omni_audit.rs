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

use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::services::omni_audit_service::OmniAuditMessage;
use crate::services::sensitive_action_alert::SensitiveActionAlert;
use crate::utils::app_state::AppState;

pub async fn omni_audit_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();

    // 提取请求信息
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
    let x_forwarded_for = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        // P2 7-11 修复：X-Forwarded-For 可能含多个 IP（"client, proxy1, proxy2"），
        // 取首段（最原始客户端 IP）并 trim 空格，与 audit_context.rs 保持一致
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string());
    let x_real_ip = req
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // IP 地址提取（优先级：X-Real-IP > X-Forwarded-For > 连接地址）
    // P3 维度 12 修复（批次 87）：直接使用 audit_context::extract_client_ip helper，
    // 上面保留的 x_forwarded_for / x_real_ip 变量原用于日志展示，ip_address 改为 helper 统一提取
    let ip_address = if x_real_ip.is_some() || x_forwarded_for.is_some() {
        x_real_ip.or(x_forwarded_for)
    } else {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip().to_string())
    };

    // 获取用户信息
    // 未认证请求的 user_id 为 None，避免与系统用户（id=0）混淆
    let user_id = req.extensions().get::<AuthContext>().map(|ctx| ctx.user_id);
    let username = req
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.username.clone())
        .unwrap_or_default();

    // 读取请求体（仅对 POST/PUT/PATCH 请求）
    let (req, request_body) = if method == "POST" || method == "PUT" || method == "PATCH" {
        let (parts, body) = req.into_parts();
        // 批次 397 修复：body 读取失败时记录 warn 日志而非静默回退空字节
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

        // 重新构建请求
        let req = Request::from_parts(parts, Body::from(body_bytes));

        // P1 7-5 修复：敏感路径请求体脱敏
        // 修复背景：原完整记录请求体，change_password / reset_password / create_user /
        // setup_totp / enable_totp 等敏感路径的请求体含明文密码，被写入 omni_audit_logs.request_body，
        // 数据库泄露即可获取所有用户密码。
        // 修复方案：维护敏感路径匹配，命中时 request_body 脱敏为 "[REDACTED]"，
        // 保留审计痕迹但不泄露明文密码。
        let is_sensitive_path = is_sensitive_request_body_path(&uri);
        let body_for_audit = if is_sensitive_path {
            "[REDACTED]".to_string()
        } else {
            body_str
        };

        // 截断过长的请求体
        // P2 7-12 修复：原 &body_for_audit[..5000] 按字节切片，切到 UTF-8 多字节字符
        // 中间会 panic。改为 chars().take(5000).collect() 按 Unicode 字符截断。
        let truncated_body = if body_for_audit.chars().count() > 5000 {
            format!("{}...", body_for_audit.chars().take(5000).collect::<String>())
        } else {
            body_for_audit
        };

        (req, Some(truncated_body))
    } else {
        (req, None)
    };

    // 生成 Trace ID
    let trace_id = uuid::Uuid::new_v4().to_string();

    // 记录请求开始日志
    tracing::info!(
        "[{}] {} {} 开始 | 用户: {}({}) | IP: {} | Query: {} | Content-Type: {}",
        trace_id,
        method,
        uri,
        username,
        user_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "anonymous".to_string()),
        ip_address.as_deref().unwrap_or("unknown"),
        query_string,
        content_type.as_deref().unwrap_or("-")
    );

    // 放行请求并获取响应
    let response = next.run(req).await;

    let duration_ms = start_time.elapsed().as_millis() as i32;
    let duration_secs = start_time.elapsed().as_secs_f64();
    let status_code = response.status();
    let status_str = if status_code.is_success() {
        "SUCCESS".to_string()
    } else if status_code == StatusCode::UNAUTHORIZED || status_code == StatusCode::FORBIDDEN {
        "DENIED".to_string()
    } else {
        "FAILED".to_string()
    };

    // 读取响应体内容（限制大小为 10KB）
    let (parts, body) = response.into_parts();
    // 批次 397 修复：响应 body 读取失败时记录 warn 日志而非静默回退空字节
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
    let response_content_type = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 重新构建响应
    let response = Response::from_parts(parts, Body::from(body_bytes));

    // 记录请求完成日志
    if status_code.is_success() {
        tracing::info!(
            "[{}] {} {} 完成 | 状态: {} | 耗时: {:.3}s | 响应大小: {} bytes",
            trace_id,
            method,
            uri,
            status_code.as_u16(),
            duration_secs,
            response_body.len()
        );
    } else {
        tracing::warn!(
            "[{}] {} {} 失败 | 状态: {} | 耗时: {:.3}s | 错误: {}",
            trace_id,
            method,
            uri,
            status_code.as_u16(),
            duration_secs,
            // P2 7-12 修复：原 &response_body[..500] 按字节切片可能 panic，改为 chars 截断
            if response_body.chars().count() > 500 {
                response_body.chars().take(500).collect::<String>()
            } else {
                response_body.clone()
            }
        );
    }

    // 发送审计日志
    // P0 8-1 修复：全局挂载后需跳过以下路径避免敏感信息泄露与无意义审计：
    // 1. PUBLIC_PATHS（登录/刷新含密码字段，不应进审计日志）
    // 2. /metrics、/health（高频探针，无业务价值）
    // 3. /swagger-ui、/api-docs（API 文档静态资源）
    // 4. /static（前端静态资源）
    let should_skip_audit = is_public_path(&uri)
        || uri.starts_with("/metrics")
        || uri.starts_with("/health")
        || uri.starts_with("/swagger-ui")
        || uri.starts_with("/api-docs")
        || uri.starts_with("/static");

    if !should_skip_audit {
        // 根据 URI 推断模块
        let module = infer_module_from_path(&uri);

        // 检查是否为敏感操作
        let _sensitive_action = SensitiveActionAlert::check_and_alert(
            &method,
            &module,
            user_id.unwrap_or(0),
            &username,
            ip_address.as_deref(),
        );

        // 截断过长的响应内容
        // P2 7-12 修复：原 &response_body[..2000] 按字节切片可能 panic，改为 chars 截断
        let truncated_response = if response_body.chars().count() > 2000 {
            format!("{}...", response_body.chars().take(2000).collect::<String>())
        } else {
            response_body.clone()
        };

        // 提取资源ID（从路径参数）
        let resource_id = extract_resource_id(&uri);

        state.omni_audit.log(OmniAuditMessage {
            trace_id: trace_id.clone(),
            user_id,
            username: Some(username),
            event_type: "API_CALL".to_string(),
            event_name: format!("{} {}", method, uri),
            resource: uri.clone(),
            action: method.clone(),
            resource_type: Some(module),
            resource_id,
            resource_name: None,
            description: Some(format!("{} {} - {}", method, uri, status_code.as_u16())),
            payload: Some(serde_json::json!({
                "status_code": status_code.as_u16(),
                "query_string": query_string,
                "request_body": request_body,
                "response_body": truncated_response,
                "response_content_type": response_content_type,
                "duration_ms": duration_ms,
                "duration_secs": duration_secs,
                "ip_address": ip_address,
                "user_agent": user_agent,
                "referer": referer,
                "content_type": content_type,
                "accept": accept,
                "response_size_bytes": response_body.len(),
            })),
            ip_address,
            user_agent,
            request_method: Some(method),
            request_path: Some(uri),
            request_body,
            duration_ms,
            status: status_str,
            error_msg: if !status_code.is_success() {
                Some(truncated_response)
            } else {
                None
            },
            old_value: None,
            new_value: None,
        });
    }

    Ok(response)
}

/// 判断请求路径是否为敏感路径（请求体含密码等敏感信息，需脱敏）
///
/// P1 7-5 修复：以下路径的请求体含明文密码/TOTP 密钥等敏感信息，
/// 审计日志中需脱敏为 "[REDACTED]"，防止数据库泄露时暴露用户密码。
///
/// 匹配规则：
/// - change-password / reset_password：含新旧密码
/// - create_user / update_user：含初始密码
/// - login / refresh：已通过 is_public_path 跳过审计，此处冗余匹配作双保险
/// - setup-totp / enable-totp / verify-totp：含 TOTP 验证码（短期敏感）
fn is_sensitive_request_body_path(uri: &str) -> bool {
    // 标准化：去除 query string，统一小写匹配
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
    // 精确匹配或 ends_with 匹配（兼容不同前缀）
    SENSITIVE_PATTERNS
        .iter()
        .any(|p| path == *p || path.ends_with(p))
}

/// 根据请求路径推断模块名称
fn infer_module_from_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 {
        // /api/v1/erp/xxx -> xxx
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
    // 尝试提取最后一段数字作为资源ID
    parts.last().and_then(|last| {
        if last.chars().all(|c| c.is_numeric()) && !last.is_empty() {
            Some(last.to_string())
        } else {
            None
        }
    })
}
