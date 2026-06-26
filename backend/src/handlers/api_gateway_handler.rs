//! API 网关管理 handler
//!
//! 技术债务修复（2026-06-26）：
//! 前端 api-gateway.ts 调用 14 个端点，后端原仅实现 keys 的 create/list/revoke，
//! 且路由前缀不匹配。本文件补齐所有端点：
//! - endpoints/logs/stats：当前返回空数据 + TODO 消息（无对应数据库表）
//! - keys CRUD：复用 api_key_handler 已有实现
//! - keys get/update/regenerate：返回 TODO 消息

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

// 复用 api_key_handler 已有实现：keys 的 list / create / delete（= revoke）
pub use crate::handlers::api_key_handler::{create_api_key, list_api_keys};
pub use crate::handlers::api_key_handler::revoke_api_key as delete_api_key;

// ============== endpoints CRUD（无数据库表，返回占位数据） ==============

/// 列出 API 端点
///
/// 技术债务：当前无 api_endpoints 数据库表，返回空列表 + TODO 消息。
pub async fn list_api_endpoints(
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<Value>>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Vec::new(),
        "API 端点管理功能开发中",
    )))
}

/// 获取单个 API 端点
///
/// 技术债务：当前无 api_endpoints 数据库表，返回 TODO 消息。
pub async fn get_api_endpoint(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 端点管理功能开发中",
    )))
}

/// 创建 API 端点
///
/// 技术债务：当前无 api_endpoints 数据库表，返回 TODO 消息。
pub async fn create_api_endpoint(
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 端点管理功能开发中",
    )))
}

/// 更新 API 端点
///
/// 技术债务：当前无 api_endpoints 数据库表，返回 TODO 消息。
pub async fn update_api_endpoint(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 端点管理功能开发中",
    )))
}

/// 删除 API 端点
///
/// 技术债务：当前无 api_endpoints 数据库表，返回 TODO 消息。
pub async fn delete_api_endpoint(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        (),
        "API 端点管理功能开发中",
    )))
}

// ============== logs 查询（无数据库表，返回占位数据） ==============

/// 列出 API 日志
///
/// 技术债务：当前无 api_logs 数据库表，返回空列表 + TODO 消息。
pub async fn list_api_logs(
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<Value>>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Vec::new(),
        "API 日志查询功能开发中",
    )))
}

/// 获取单条 API 日志
///
/// 技术债务：当前无 api_logs 数据库表，返回 TODO 消息。
pub async fn get_api_log(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 日志查询功能开发中",
    )))
}

// ============== stats ==============

/// 获取 API 网关统计数据
///
/// 技术债务：endpoints/logs 暂无数据库表，统计值固定为 0。
pub async fn get_api_stats(
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success(json!({
        "total_endpoints": 0,
        "total_keys": 0,
        "total_logs": 0,
    }))))
}

// ============== keys get / update / regenerate（TODO 占位） ==============

/// 获取单个 API 密钥详情
///
/// 技术债务：api_key_handler 未提供 get_by_id 接口，暂返回 TODO 消息。
pub async fn get_api_key(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 密钥详情/更新/重新生成功能开发中",
    )))
}

/// 更新 API 密钥
///
/// 技术债务：api_key_handler 未提供 update 接口，暂返回 TODO 消息。
pub async fn update_api_key(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 密钥详情/更新/重新生成功能开发中",
    )))
}

/// 重新生成 API 密钥
///
/// 技术债务：api_key_handler 未提供 regenerate 接口，暂返回 TODO 消息。
pub async fn regenerate_api_key(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(_id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    Ok(Json(ApiResponse::success_with_message(
        Value::Null,
        "API 密钥详情/更新/重新生成功能开发中",
    )))
}
