use axum::extract::ConnectInfo;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::time::Instant;

use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_service::OmniAuditMessage;
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
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // IP 地址提取
    let ip_address = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip().to_string());

    // 获取用户信息
    let user_id = req
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.user_id)
        .unwrap_or(0);
    let username = req
        .extensions()
        .get::<AuthContext>()
        .map(|ctx| ctx.username.clone());

    // 生成 Trace ID
    let trace_id = uuid::Uuid::new_v4().to_string();

    // 放行请求
    let response = next.run(req).await;

    let duration_ms = start_time.elapsed().as_millis() as i32;
    let status_code = response.status();
    let status_str = if status_code.is_success() {
        "SUCCESS".to_string()
    } else if status_code == StatusCode::UNAUTHORIZED || status_code == StatusCode::FORBIDDEN {
        "DENIED".to_string()
    } else {
        "FAILED".to_string()
    };

    // 发送审计日志
    // 忽略一些高频无用接口，比如 prometheus metrics, health check
    if !uri.starts_with("/metrics") && !uri.starts_with("/health") {
        // 根据 URI 推断模块
        let module = infer_module_from_path(&uri);
        
        state.omni_audit.log(OmniAuditMessage {
            trace_id,
            user_id,
            username,
            event_type: "API_CALL".to_string(),
            event_name: format!("{} {}", method, uri),
            resource: uri.clone(),
            action: method.clone(),
            resource_type: Some(module),
            resource_id: None,
            resource_name: None,
            description: Some(format!("{} {}", method, uri)),
            payload: Some(serde_json::json!({
                "status_code": status_code.as_u16(),
                "query_string": query_string,
            })),
            ip_address,
            user_agent,
            request_method: Some(method),
            request_path: Some(uri),
            request_body: None,
            duration_ms,
            status: status_str,
            error_msg: None,
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
