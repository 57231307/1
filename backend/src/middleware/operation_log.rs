use chrono::Utc;
// 操作日志中间件
// 自动记录用户的 HTTP 请求操作

use crate::services::operation_log_service::OperationLogService;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;


/// 从请求头中提取用户信息
///
/// 注意：这里简化处理，实际需要解析 JWT Token
fn extract_user_info(headers: &axum::http::HeaderMap) -> (Option<i32>, Option<String>) {
    // 待实现(v1.1): 从请求上下文中解析 JWT Token 提取操作人 ID
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
