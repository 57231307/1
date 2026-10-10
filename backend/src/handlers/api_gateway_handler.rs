//! API 网关管理 handler。
//!
//! 提供 API 端点（`api_endpoints` 表）的增删改查，以及端点维度的聚合统计。
//!
//! 字段映射说明：后端 model 字段名与前端 TypeScript 接口存在差异，
//! handler 层通过 serde_json::Value 转换为前端期望的结构。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::api_endpoint;
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============== DTO ==============

/// 查询参数（支持分页 + 关键词 + 状态过滤）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ApiGwQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub method: Option<String>,
}

/// 创建/更新 API 端点请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
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
    /// 废弃时间
    pub deprecated_at: Option<String>,
    /// 计划下线时间
    pub sunset_at: Option<String>,
    /// 废弃原因说明
    pub deprecation_note: Option<String>,
}

// ============== 辅助函数 ==============

/// 校验 rate_limit 范围（0-10000 次/分钟），负值或超限返回 400
fn validate_rate_limit(v: Option<i32>) -> Result<Option<i32>, AppError> {
    if let Some(rl) = v {
        if !(0..=10000).contains(&rl) {
            return Err(AppError::validation_displayable(format!(
                "rate_limit 必须在 0-10000 之间，当前: {}",
                rl
            )));
        }
    }
    Ok(v)
}

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
        "deprecated_at": m.deprecated_at.map(|d| d.to_rfc3339()),
        "sunset_at": m.sunset_at.map(|d| d.to_rfc3339()),
        "deprecation_note": m.deprecation_note.unwrap_or_default(),
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

    // 检查端点是否已废弃，添加 deprecation 标记
    let mut json_response = endpoint_to_json(m.clone());
    if m.deprecated_at.is_some() || m.sunset_at.is_some() {
        // 在 JSON 响应中添加 deprecation 信息
        if let Some(obj) = json_response.as_object_mut() {
            obj.insert("is_deprecated".to_string(), json!(true));
        }
    }

    Ok(Json(ApiResponse::success(json_response)))
}

/// POST /api-gateway/endpoints — 创建 API 端点
#[axum::debug_handler]
pub async fn create_api_endpoint(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<UpsertApiEndpointRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let rate_limit = validate_rate_limit(req.rate_limit)?;
    let path = req
        .path
        .ok_or_else(|| AppError::validation_displayable("path 为必填项"))?;
    let method = req
        .method
        .ok_or_else(|| AppError::validation_displayable("method 为必填项"))?;

    // 唯一性检查（path + method）
    // 显式检查仅作友好提示；并发场景下 TOCTOU 由数据库唯一约束
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
        status: sea_orm::Set(
            req.status
                .unwrap_or_else(|| master_data::ACTIVE.to_string()),
        ),
        rate_limit: sea_orm::Set(rate_limit.unwrap_or(0)),
        timeout: sea_orm::Set(req.timeout.unwrap_or(30000)),
        authentication: sea_orm::Set(req.authentication.unwrap_or(true)),
        authorization: sea_orm::Set(req.authorization),
        request_schema: sea_orm::Set(req.request_schema),
        response_schema: sea_orm::Set(req.response_schema),
        version: sea_orm::Set(req.version.or_else(|| Some("v1".to_string()))),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        deprecated_at: sea_orm::Set(
            req.deprecated_at
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
                .map(|d| d.with_timezone(&chrono::Utc)),
        ),
        sunset_at: sea_orm::Set(
            req.sunset_at
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
                .map(|d| d.with_timezone(&chrono::Utc)),
        ),
        deprecation_note: sea_orm::Set(req.deprecation_note),
        ..Default::default()
    };

    // catch 唯一约束冲突（并发场景下显式 find 通过但 insert 冲突）
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
    let rate_limit = validate_rate_limit(req.rate_limit)?;
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
    if let Some(rate_limit) = rate_limit {
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
    if let Some(deprecated_at) = req.deprecated_at {
        active.deprecated_at = sea_orm::Set(
            chrono::DateTime::parse_from_rfc3339(&deprecated_at)
                .ok()
                .map(|d| d.with_timezone(&chrono::Utc)),
        );
    }
    if let Some(sunset_at) = req.sunset_at {
        active.sunset_at = sea_orm::Set(
            chrono::DateTime::parse_from_rfc3339(&sunset_at)
                .ok()
                .map(|d| d.with_timezone(&chrono::Utc)),
        );
    }
    if let Some(deprecation_note) = req.deprecation_note {
        active.deprecation_note = sea_orm::Set(Some(deprecation_note));
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
    Ok(Json(ApiResponse::success_with_message((), "端点删除成功")))
}

// ============== stats ==============

/// GET /api-gateway/stats — 获取 API 网关统计数据（端点维度）
pub async fn get_api_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let total_endpoints = api_endpoint::Entity::find().count(&*state.db).await?;
    let active_endpoints = api_endpoint::Entity::find()
        .filter(api_endpoint::Column::Status.eq(master_data::ACTIVE))
        .count(&*state.db)
        .await?;
    let inactive_endpoints = api_endpoint::Entity::find()
        .filter(api_endpoint::Column::Status.eq(master_data::INACTIVE))
        .count(&*state.db)
        .await?;

    Ok(Json(ApiResponse::success(json!({
        "total_endpoints": total_endpoints,
        "active_endpoints": active_endpoints,
        "inactive_endpoints": inactive_endpoints,
    }))))
}
