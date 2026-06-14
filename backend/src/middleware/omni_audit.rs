use axum::extract::ConnectInfo;
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::time::Instant;

use crate::middleware::auth_context::AuthContext;
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
        .map(|s| s.to_string());
    let x_real_ip = req
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // IP 地址提取（优先级：X-Real-IP > X-Forwarded-For > 连接地址）
    let ip_address = x_real_ip.or(x_forwarded_for).or_else(|| {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip().to_string())
    });

    // 获取用户信息
    // 未认证请求的 user_id 为 None，避免与系统用户（id=0）混淆
    let user_id = req.extensions().get::<AuthContext>().map(|ctx| ctx.user_id);
    let username = req
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.username.clone())
        .unwrap_or_default();
    let tenant_id = req
        .extensions()
        .get::<AuthContext>()
        .and_then(|ctx| ctx.tenant_id);

    // 读取请求体（仅对 POST/PUT/PATCH 请求）
    let (req, request_body) = if method == "POST" || method == "PUT" || method == "PATCH" {
        let (parts, body) = req.into_parts();
        let body_bytes = to_bytes(body, 50 * 1024).await.unwrap_or_default();
        let body_str = String::from_utf8_lossy(&body_bytes).to_string();

        // 重新构建请求
        let req = Request::from_parts(parts, Body::from(body_bytes));

        // 截断过长的请求体
        let truncated_body = if body_str.len() > 5000 {
            format!("{}...", &body_str[..5000])
        } else {
            body_str
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
        user_id.map(|u| u.to_string()).unwrap_or_else(|| "anonymous".to_string()),
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
    let body_bytes = to_bytes(body, 10 * 1024).await.unwrap_or_default();
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
            if response_body.len() > 500 {
                &response_body[..500]
            } else {
                &response_body
            }
        );
    }

    // 发送审计日志
    // 忽略一些高频无用接口，比如 prometheus metrics, health check
    if !uri.starts_with("/metrics") && !uri.starts_with("/health") {
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
        let truncated_response = if response_body.len() > 2000 {
            format!("{}...", &response_body[..2000])
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
                "tenant_id": tenant_id,
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
