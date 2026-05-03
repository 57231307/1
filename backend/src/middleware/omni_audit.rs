use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::time::Instant;
use axum::extract::ConnectInfo;
use std::net::SocketAddr;

use crate::utils::app_state::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::omni_audit_service::OmniAuditMessage;

pub async fn omni_audit_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();

    // 提取请求信息
    let method = req.method().to_string();
    let uri = req.uri().path().to_string();
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // IP 地址提取 (简化)
    let ip_address = req.extensions().get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip().to_string());

    // 为了获取 user_id，我们需要看 extension 里有没有 AuthContext
    // 注意：如果这个中间件在 auth 之后，就能取到
    // 如果在 auth 之前或者 public 路由，就取不到
    let user_id = req.extensions().get::<AuthContext>().map(|ctx| ctx.user_id).unwrap_or(0);

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
        state.omni_audit.log(OmniAuditMessage {
            trace_id,
            user_id,
            event_type: "API_CALL".to_string(),
            event_name: format!("{} {}", method, uri),
            resource: uri,
            action: method,
            payload: Some(serde_json::json!({
                "status_code": status_code.as_u16(),
            })),
            ip_address,
            user_agent,
            duration_ms,
            status: status_str,
            error_msg: None,
        });
    }

    Ok(response)
}
