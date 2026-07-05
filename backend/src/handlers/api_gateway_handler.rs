//! API 网关管理 handler（批次 91 P0-1 全面实现）
//!
//! 前端 api-gateway.ts 调用 14 个端点，本文件提供全部实现：
//! - endpoints CRUD：基于 api_endpoints 表
//! - logs 查询：复用 log_api_accesses 表
//! - keys CRUD + get/update/regenerate：复用 api_keys 表 + ApiKeyService
//! - stats：聚合查询 api_endpoints + api_keys + log_api_accesses
//!
//! 字段映射说明：后端 model 字段名与前端 TypeScript 接口存在差异，
//! handler 层通过 serde_json::Value 转换为前端期望的结构。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::middleware::auth_context::AuthContext;
use crate::models::{api_endpoint, log_api_access};
use crate::services::api_key_service::ApiKeyService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============== DTO ==============

/// 查询参数（支持分页 + 关键词 + 状态过滤）
#[derive(Debug, Deserialize)]
pub struct ApiGwQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub method: Option<String>,
}

/// 创建/更新 API 端点请求 DTO
#[derive(Debug, Deserialize)]
pub struct UpsertApiEndpointRequest {
    pub path: Option<String>,
    pub method: Option<String>,
    pub description: Option<String>,
    pub module: Option<String>,
    pub status: Option<String>,
    pub rate_limit: Option<i32>,
    pub timeout: Option<i32>,
    pub authentication: Option<bool>,
    pub authorization: Option<Value>,
    pub request_schema: Option<Value>,
    pub response_schema: Option<Value>,
    pub version: Option<String>,
}

/// 更新 API 密钥请求 DTO
#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyGwRequest {
    pub key_name: Option<String>,
    // 批次 95 CI 修复：api_keys 表无 description 列，保留占位待 schema 扩展后接入
    #[allow(dead_code)]
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub rate_limit: Option<i32>,
    pub expires_at: Option<String>,
    pub status: Option<String>,
}

// ============== 辅助函数 ==============

fn page_offset(query: &ApiGwQuery) -> (u64, u64) {
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    ((page - 1) * page_size, page_size)
}

/// 将 api_endpoint::Model 转换为前端期望的 JSON 结构
fn endpoint_to_json(m: api_endpoint::Model) -> Value {
    json!({
        "id": m.id,
        "path": m.path,
        "method": m.method,
        "description": m.description.unwrap_or_default(),
        "module": m.module.unwrap_or_default(),
        "status": m.status,
        "rate_limit": m.rate_limit,
        "timeout": m.timeout,
        "authentication": m.authentication,
        "authorization": m.authorization.unwrap_or_else(|| json!([])),
        "request_schema": m.request_schema.unwrap_or_else(|| json!({})),
        "response_schema": m.response_schema.unwrap_or_else(|| json!({})),
        "version": m.version.unwrap_or_else(|| "v1".to_string()),
        "created_at": m.created_at.to_rfc3339(),
        "updated_at": m.updated_at.to_rfc3339(),
    })
}

/// 将 log_api_access::Model 转换为前端期望的 JSON 结构
fn log_to_json(m: log_api_access::Model) -> Value {
    let ip = m.ip_address.unwrap_or_default();
    json!({
        "id": m.id,
        "endpoint_id": null,
        "endpoint_path": m.path,
        "path": m.path,
        "method": m.method,
        "request_body": m.request_body.unwrap_or_default(),
        "response_body": m.error_message.unwrap_or_default(),
        "status_code": m.status_code.unwrap_or(0),
        "response_time": m.execution_time,
        "duration": m.execution_time,
        "ip_address": &ip,
        "client_ip": &ip,
        "user_agent": m.user_agent.unwrap_or_default(),
        "user_id": m.user_id.unwrap_or(0),
        "user_name": m.username.unwrap_or_default(),
        "api_key_name": null,
        "created_at": m.created_at.to_rfc3339(),
    })
}

/// 将 api_key::Model 转换为前端期望的 JSON 结构
///
/// 批次 112 P1-9：created_by 直接从 model.created_by 读取（migration m0039 新增列），
/// 不再需要调用方传 0 占位。历史数据 created_by 为 NULL 时返回 0 保持前端兼容。
fn key_to_json(m: &crate::models::api_key::Model) -> Value {
    // permissions 字段为 JSON 字符串，解析为 string[]
    let permissions: Value = m
        .permissions
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| json!([]));

    let status = if !m.is_active {
        "inactive"
    } else if m
        .expires_at
        .as_ref()
        .map(|e| *e < Utc::now())
        .unwrap_or(false)
    {
        "expired"
    } else {
        "active"
    };

    json!({
        "id": m.id,
        "key_name": &m.name,
        "api_key": &m.key_prefix,
        "app_id": &m.key_prefix,
        "description": "",
        "permissions": permissions,
        "rate_limit": m.rate_limit_per_minute,
        "expires_at": m.expires_at.as_ref().map(|d| d.to_rfc3339()).unwrap_or_default(),
        "status": status,
        "created_by": m.created_by.unwrap_or(0),
        "created_by_name": "",
        "created_at": m.created_at.to_rfc3339(),
        "last_used_at": m.last_used_at.as_ref().map(|d| d.to_rfc3339()).unwrap_or_default(),
    })
}

// ============== endpoints CRUD ==============

/// GET /api-gateway/endpoints — 列出 API 端点（分页）
pub async fn list_api_endpoints(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ApiGwQuery>,
) -> Result<Json<ApiResponse<Vec<Value>>>, AppError> {
    let (offset, limit) = page_offset(&query);

    let mut sel = api_endpoint::Entity::find();
    if let Some(ref kw) = query.keyword {
        sel = sel.filter(
            api_endpoint::Column::Path
                .contains(kw)
                .or(api_endpoint::Column::Description.contains(kw)),
        );
    }
    if let Some(ref status) = query.status {
        sel = sel.filter(api_endpoint::Column::Status.eq(status));
    }
    if let Some(ref method) = query.method {
        sel = sel.filter(api_endpoint::Column::Method.eq(method));
    }

    let total = sel.clone().count(&*state.db).await?;
    let rows = sel
        .order_by_desc(api_endpoint::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(&*state.db)
        .await?;

    let data: Vec<Value> = rows.into_iter().map(endpoint_to_json).collect();
    Ok(Json(ApiResponse {
        code: Some(200),
        message: Some("success".to_string()),
        data: Some(data),
        total: Some(total),
    }))
}

/// GET /api-gateway/endpoints/:id — 获取单个 API 端点
pub async fn get_api_endpoint(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let m = api_endpoint::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("API 端点 {} 不存在", id)))?;
    Ok(Json(ApiResponse::success(endpoint_to_json(m))))
}

/// POST /api-gateway/endpoints — 创建 API 端点
#[axum::debug_handler]
pub async fn create_api_endpoint(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<UpsertApiEndpointRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let path = req
        .path
        .ok_or_else(|| AppError::validation("path 为必填项"))?;
    let method = req
        .method
        .ok_or_else(|| AppError::validation("method 为必填项"))?;

    // 唯一性检查（path + method）
    // 批次 95 P3-10 修复：显式检查仅作友好提示；并发场景下 TOCTOU 由数据库唯一约束
    // uk_api_endpoints_path_method 兜底（见 migrations/20260703000005_create_api_endpoints/up.sql），
    // insert 阶段会 catch 该约束冲突并转为业务错误。
    let existing = api_endpoint::Entity::find()
        .filter(api_endpoint::Column::Path.eq(&path))
        .filter(api_endpoint::Column::Method.eq(&method))
        .one(&*state.db)
        .await?;
    if existing.is_some() {
        return Err(AppError::business("该路径+方法的端点已存在"));
    }

    let now = Utc::now();
    let active_model = api_endpoint::ActiveModel {
        path: sea_orm::Set(path),
        method: sea_orm::Set(method),
        description: sea_orm::Set(req.description),
        module: sea_orm::Set(req.module),
        status: sea_orm::Set(req.status.unwrap_or_else(|| "active".to_string())),
        rate_limit: sea_orm::Set(req.rate_limit.unwrap_or(0)),
        timeout: sea_orm::Set(req.timeout.unwrap_or(30000)),
        authentication: sea_orm::Set(req.authentication.unwrap_or(true)),
        authorization: sea_orm::Set(req.authorization),
        request_schema: sea_orm::Set(req.request_schema),
        response_schema: sea_orm::Set(req.response_schema),
        version: sea_orm::Set(req.version.or_else(|| Some("v1".to_string()))),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        ..Default::default()
    };

    // 批次 95 P3-10：catch 唯一约束冲突（并发场景下显式 find 通过但 insert 冲突）
    // 仅匹配特定约束名 uk_api_endpoints_path_method，避免吞掉其他系统错误
    let m = match active_model.insert(&*state.db).await {
        Ok(m) => m,
        Err(err) => {
            let err_str = err.to_string();
            if err_str.contains("uk_api_endpoints_path_method") {
                return Err(AppError::business("该路径+方法的端点已存在"));
            }
            return Err(err.into());
        }
    };
    Ok(Json(ApiResponse::success_with_message(
        endpoint_to_json(m),
        "端点创建成功",
    )))
}

/// PUT /api-gateway/endpoints/:id — 更新 API 端点
#[axum::debug_handler]
pub async fn update_api_endpoint(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpsertApiEndpointRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let m = api_endpoint::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("API 端点 {} 不存在", id)))?;

    let mut active: api_endpoint::ActiveModel = m.into();
    if let Some(path) = req.path {
        active.path = sea_orm::Set(path);
    }
    if let Some(method) = req.method {
        active.method = sea_orm::Set(method);
    }
    if let Some(description) = req.description {
        active.description = sea_orm::Set(Some(description));
    }
    if let Some(module) = req.module {
        active.module = sea_orm::Set(Some(module));
    }
    if let Some(status) = req.status {
        active.status = sea_orm::Set(status);
    }
    if let Some(rate_limit) = req.rate_limit {
        active.rate_limit = sea_orm::Set(rate_limit);
    }
    if let Some(timeout) = req.timeout {
        active.timeout = sea_orm::Set(timeout);
    }
    if let Some(authentication) = req.authentication {
        active.authentication = sea_orm::Set(authentication);
    }
    if let Some(authorization) = req.authorization {
        active.authorization = sea_orm::Set(Some(authorization));
    }
    if let Some(request_schema) = req.request_schema {
        active.request_schema = sea_orm::Set(Some(request_schema));
    }
    if let Some(response_schema) = req.response_schema {
        active.response_schema = sea_orm::Set(Some(response_schema));
    }
    if let Some(version) = req.version {
        active.version = sea_orm::Set(Some(version));
    }
    active.updated_at = sea_orm::Set(Utc::now());

    let updated = active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        endpoint_to_json(updated),
        "端点更新成功",
    )))
}

/// DELETE /api-gateway/endpoints/:id — 删除 API 端点
pub async fn delete_api_endpoint(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    api_endpoint::Entity::delete_by_id(id)
        .exec(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "端点删除成功",
    )))
}

// ============== logs 查询（复用 log_api_accesses 表） ==============

/// GET /api-gateway/logs — 列出 API 调用日志（分页）
pub async fn list_api_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ApiGwQuery>,
) -> Result<Json<ApiResponse<Vec<Value>>>, AppError> {
    let (offset, limit) = page_offset(&query);

    let mut sel = log_api_access::Entity::find();
    if let Some(ref kw) = query.keyword {
        sel = sel.filter(
            log_api_access::Column::Path
                .contains(kw)
                .or(log_api_access::Column::Username.contains(kw)),
        );
    }
    if let Some(ref method) = query.method {
        sel = sel.filter(log_api_access::Column::Method.eq(method));
    }
    // status 参数：前端用 2xx/4xx/5xx 区间过滤
    if let Some(ref status) = query.status {
        if let Ok(code_prefix) = status.parse::<i32>() {
            let lower = code_prefix * 100;
            let upper = lower + 99;
            sel = sel.filter(log_api_access::Column::StatusCode.between(lower, upper));
        }
    }

    let total = sel.clone().count(&*state.db).await?;
    let rows = sel
        .order_by_desc(log_api_access::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(&*state.db)
        .await?;

    let data: Vec<Value> = rows.into_iter().map(log_to_json).collect();
    Ok(Json(ApiResponse {
        code: Some(200),
        message: Some("success".to_string()),
        data: Some(data),
        total: Some(total),
    }))
}

/// GET /api-gateway/logs/:id — 获取单条 API 日志
pub async fn get_api_log(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let m = log_api_access::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("API 日志 {} 不存在", id)))?;
    Ok(Json(ApiResponse::success(log_to_json(m))))
}

// ============== stats ==============

/// GET /api-gateway/stats — 获取 API 网关统计数据
pub async fn get_api_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let total_endpoints = api_endpoint::Entity::find().count(&*state.db).await?;
    let active_endpoints = api_endpoint::Entity::find()
        .filter(api_endpoint::Column::Status.eq("active"))
        .count(&*state.db)
        .await?;
    let inactive_endpoints = api_endpoint::Entity::find()
        .filter(api_endpoint::Column::Status.eq("inactive"))
        .count(&*state.db)
        .await?;

    let total_keys = crate::models::api_key::Entity::find()
        .count(&*state.db)
        .await?;
    let active_keys = crate::models::api_key::Entity::find()
        .filter(crate::models::api_key::Column::IsActive.eq(true))
        .count(&*state.db)
        .await?;

    let total_requests = log_api_access::Entity::find().count(&*state.db).await?;
    let total_errors = log_api_access::Entity::find()
        .filter(log_api_access::Column::StatusCode.gte(400))
        .count(&*state.db)
        .await?;

    // 平均响应时间：取最近 1000 条日志的平均 execution_time
    let recent_logs = log_api_access::Entity::find()
        .order_by_desc(log_api_access::Column::CreatedAt)
        .limit(1000)
        .all(&*state.db)
        .await?;
    let avg_response_time_ms = if recent_logs.is_empty() {
        0.0
    } else {
        let sum: i64 = recent_logs.iter().map(|l| l.execution_time).sum();
        sum as f64 / recent_logs.len() as f64
    };

    Ok(Json(ApiResponse::success(json!({
        "total_endpoints": total_endpoints,
        "active_endpoints": active_endpoints,
        "inactive_endpoints": inactive_endpoints,
        "total_keys": total_keys,
        "active_keys": active_keys,
        "total_requests": total_requests,
        "total_errors": total_errors,
        "avg_response_time_ms": avg_response_time_ms.round() as i64,
    }))))
}

// ============== keys CRUD + get/update/regenerate ==============
//
// list/create/delete 原通过 pub use 复用 api_key_handler，但前端期望的字段名
// 与 api_key_handler::ApiKeyResponse 不一致（key_name vs name 等）。
// 批次 103 P1-7 修复：api_key_handler 模块已删除（死代码，业务全部迁移到 api_gateway_handler）。
// 批次 91 P0-1 重新实现 keys 端点，统一返回前端期望的结构。

/// GET /api-gateway/keys — 列出 API 密钥（分页）
pub async fn list_api_keys(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ApiGwQuery>,
) -> Result<Json<ApiResponse<Vec<Value>>>, AppError> {
    let (offset, limit) = page_offset(&query);

    let mut sel = crate::models::api_key::Entity::find();
    if let Some(ref kw) = query.keyword {
        sel = sel.filter(crate::models::api_key::Column::Name.contains(kw));
    }
    // status 参数：active/inactive
    if let Some(ref status) = query.status {
        match status.as_str() {
            "active" => {
                sel = sel.filter(crate::models::api_key::Column::IsActive.eq(true));
            }
            "inactive" => {
                sel = sel.filter(crate::models::api_key::Column::IsActive.eq(false));
            }
            _ => {}
        }
    }

    let total = sel.clone().count(&*state.db).await?;
    let rows = sel
        .order_by_desc(crate::models::api_key::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(&*state.db)
        .await?;

    // 批次 112 P1-9：created_by 从 model.created_by 读取（migration m0039 新增列）
    let data: Vec<Value> = rows.iter().map(key_to_json).collect();
    Ok(Json(ApiResponse {
        code: Some(200),
        message: Some("success".to_string()),
        data: Some(data),
        total: Some(total),
    }))
}

/// POST /api-gateway/keys — 创建 API 密钥
#[axum::debug_handler]
pub async fn create_api_key(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApiKeyGwRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = ApiKeyService::new(state.db.clone());
    let permissions = req
        .permissions
        .as_ref()
        .map(|p| serde_json::to_string(p).unwrap_or_default());
    let rate_limit = req.rate_limit.unwrap_or(100);

    // expires_at 字符串 → expires_days 数值
    let expires_days = req.expires_at.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| (d.with_timezone(&Utc) - Utc::now()).num_days().max(1))
        }
    });

    let (model, plain_key) = service
        .create_api_key(
            &req.key_name,
            permissions.as_deref(),
            rate_limit,
            expires_days,
            // 批次 112 P1-9：注入真实创建者 user_id（migration m0039 持久化到 created_by 列）
            auth.user_id,
        )
        .await?;

    // 批次 112 P1-9：key_to_json 直接从 model.created_by 读取
    let mut data = key_to_json(&model);
    if let Some(obj) = data.as_object_mut() {
        obj.insert("plain_key".to_string(), Value::String(plain_key));
    }

    Ok(Json(ApiResponse::success_with_message(
        data,
        "密钥创建成功",
    )))
}

/// GET /api-gateway/keys/:id — 获取单个 API 密钥详情
pub async fn get_api_key(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = ApiKeyService::new(state.db.clone());
    let model = service
        .get_api_key_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("API 密钥 {} 不存在", id)))?;
    // 批次 112 P1-9：created_by 从 model.created_by 读取
    Ok(Json(ApiResponse::success(key_to_json(&model))))
}

/// PUT /api-gateway/keys/:id — 更新 API 密钥
#[axum::debug_handler]
pub async fn update_api_key(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateApiKeyGwRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = ApiKeyService::new(state.db.clone());

    // permissions: Vec<String> → JSON 字符串
    let permissions = req.permissions.map(|p| serde_json::to_string(&p).unwrap_or_default());

    // status → is_active
    let is_active = req.status.as_deref().map(|s| s == "active");

    // expires_at: ISO 字符串 → DateTime
    let expires_at = req.expires_at.as_ref().map(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .ok()
    });

    let model = service
        .update_api_key(
            id,
            req.key_name,
            permissions,
            req.rate_limit,
            expires_at,
            is_active,
        )
        .await?;

    // 批次 112 P1-9：created_by 从 model.created_by 读取
    Ok(Json(ApiResponse::success_with_message(
        key_to_json(&model),
        "密钥更新成功",
    )))
}

/// DELETE /api-gateway/keys/:id — 删除（撤销）API 密钥
pub async fn delete_api_key(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ApiKeyService::new(state.db.clone());
    service.revoke_api_key(id, Some(&state.cache)).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "密钥已撤销",
    )))
}

/// POST /api-gateway/keys/:id/regenerate — 重新生成 API 密钥
#[axum::debug_handler]
pub async fn regenerate_api_key(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let service = ApiKeyService::new(state.db.clone());
    let (model, plain_key) = service
        .regenerate_api_key(
            id,
            Some(&state.cache),
            // 批次 112 P1-9：注入重新生成操作者 user_id，更新 created_by
            auth.user_id,
        )
        .await?;

    // 批次 112 P1-9：key_to_json 直接从 model.created_by 读取
    let mut data = key_to_json(&model);
    if let Some(obj) = data.as_object_mut() {
        obj.insert("plain_key".to_string(), Value::String(plain_key));
    }

    Ok(Json(ApiResponse::success_with_message(
        data,
        "密钥已重新生成",
    )))
}

// ============== 请求 DTO ==============

/// 创建 API 密钥请求（前端 key_name/permissions[]/rate_limit/expires_at）
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyGwRequest {
    pub key_name: String,
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
    pub rate_limit: Option<i32>,
    /// 过期时间（ISO 8601 字符串，前端传 expires_at）
    pub expires_at: Option<String>,
}
