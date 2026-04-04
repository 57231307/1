use chrono::Utc;
// 操作日志中间件
// 自动记录用户的 HTTP 请求操作

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::services::operation_log_service::OperationLogService;

/// 操作日志中间件
/// 
/// 自动记录每个 HTTP 请求的操作信息，包括：
/// - 用户信息（从 Token 中提取）
/// - 请求方法、URI、IP
/// - 响应状态码
/// - 请求耗时
pub async fn operation_log_middleware(
    State(db): State<Arc<DatabaseConnection>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Utc::now();
    
    // 提取请求信息
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    // 获取客户端 IP
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .unwrap_or("unknown")
        .to_string();
    
    // 获取 User-Agent
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string());
    
    // 尝试从 Authorization 头中提取用户信息（简化版，实际需要解析 JWT）
    let (user_id, username) = extract_user_info(&headers);
    
    // 执行请求
    let response = next.run(request).await;
    
    // 计算耗时
    let end_time = Utc::now();
    let duration_ms = (end_time - start_time).num_milliseconds();
    
    // 判断操作状态
    let _status = if response.status().is_success() {
        "success"
    } else {
        "failure"
    };
    
    // 异步记录日志（不阻塞响应）
    let log_service = OperationLogService::new(db.clone());
    let module = extract_module_from_uri(&uri);
    let action = extract_action_from_method(&method);
    
    // 使用 spawn 异步记录，不阻塞主流程
    tokio::spawn(async move {
        let _ = log_service
            .log_success(
                user_id,
                username,
                &module,
                &action,
                Some(format!("{} {}", method, uri.path())),
                Some(method.to_string()),
                Some(uri.path().to_string()),
                Some(client_ip),
                user_agent,
                Some(duration_ms),
                None,
            )
            .await;
    });
    
    Ok(response)
}

/// 从请求头中提取用户信息
/// 
/// 注意：这里简化处理，实际需要解析 JWT Token
fn extract_user_info(headers: &axum::http::HeaderMap) -> (Option<i32>, Option<String>) {
    // TODO: 解析 JWT Token 获取真实用户信息
    // 这里暂时返回 None，实际使用时需要从 Token 中解析
    
    // 示例：从自定义头中获取（如果有）
    if let Some(user_id_str) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) {
        if let Ok(user_id) = user_id_str.parse::<i32>() {
            let username = headers
                .get("x-username")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            return (Some(user_id), username);
        }
    }
    
    (None, None)
}

/// 从 URI 中提取模块名
fn extract_module_from_uri(uri: &axum::http::Uri) -> String {
    let path = uri.path();
    
    // 根据路径提取模块
    if path.contains("/users") {
        "user".to_string()
    } else if path.contains("/roles") {
        "role".to_string()
    } else if path.contains("/products") {
        "product".to_string()
    } else if path.contains("/product-categories") {
        "product_category".to_string()
    } else if path.contains("/warehouses") {
        "warehouse".to_string()
    } else if path.contains("/departments") {
        "department".to_string()
    } else if path.contains("/inventory/stock") {
        "inventory_stock".to_string()
    } else if path.contains("/inventory/transfers") {
        "inventory_transfer".to_string()
    } else if path.contains("/inventory/counts") {
        "inventory_count".to_string()
    } else if path.contains("/inventory/adjustments") {
        "inventory_adjustment".to_string()
    } else if path.contains("/sales") {
        "sales".to_string()
    } else if path.contains("/finance/payments") {
        "finance_payment".to_string()
    } else if path.contains("/finance/invoices") {
        "finance_invoice".to_string()
    } else if path.contains("/dashboard") {
        "dashboard".to_string()
    } else if path.contains("/auth") {
        "auth".to_string()
    } else {
        "other".to_string()
    }
}

/// 从 HTTP 方法中提取操作类型
fn extract_action_from_method(method: &axum::http::Method) -> String {
    match method.as_str() {
        "GET" => "query".to_string(),
        "POST" => "create".to_string(),
        "PUT" => "update".to_string(),
        "PATCH" => "update".to_string(),
        "DELETE" => "delete".to_string(),
        _ => "other".to_string(),
    }
}
