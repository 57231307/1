//! 角色关系 handler（V15 P1 12.2）
//!
//! 角色继承与互斥校验 HTTP 接口：
//! - 角色继承：sales_manager 继承 sales 的所有权限
//! - 权限互斥：finance 与 sales 不能同时拥有（职责分离）

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::role_relation_service::{CreateRoleRelationRequest, RoleRelationService};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// 查询参数：关系类型过滤
#[derive(Debug, Deserialize)]
pub struct RelationTypeQuery {
    pub relation_type: Option<String>,
}

/// 检查角色互斥请求体
#[derive(Debug, Deserialize)]
pub struct CheckMutualExclusiveRequest {
    /// 用户当前已持有的角色编码列表
    pub existing_role_codes: Vec<String>,
}

/// 创建角色关系
/// POST /role-relations
pub async fn create_relation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateRoleRelationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = RoleRelationService::new(state.db.clone());
    let model = service.create_relation(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 删除角色关系
/// DELETE /role-relations/:relation_id
pub async fn delete_relation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(relation_id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = RoleRelationService::new(state.db.clone());
    service.delete_relation(relation_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        serde_json::json!({ "deleted": relation_id }),
    )?)))
}

/// 查询所有角色关系
/// GET /role-relations
pub async fn list_relations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<RelationTypeQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = RoleRelationService::new(state.db.clone());
    let list = service
        .list_relations(params.relation_type.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 查询两个角色之间的关系（含双向）
/// GET /role-relations/between/:role_a_code/:role_b_code
pub async fn get_relation_between(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path((role_a_code, role_b_code)): Path<(String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = RoleRelationService::new(state.db.clone());
    let list = service
        .get_relation_between(&role_a_code, &role_b_code)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 获取角色继承的所有子角色编码
/// GET /role-relations/inherited/:role_code
pub async fn get_inherited_role_codes(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(role_code): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = RoleRelationService::new(state.db.clone());
    let codes = service.get_inherited_role_codes(&role_code).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(codes)?)))
}

/// 检查角色互斥
/// POST /role-relations/check-mutual-exclusive/:role_code
///
/// `role_code` 为待分配的新角色编码，请求体携带用户当前已持有的角色编码列表。
/// 无互斥冲突返回成功；存在互斥冲突返回业务错误（职责分离原则）。
pub async fn check_mutual_exclusive(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(role_code): Path<String>,
    Json(req): Json<CheckMutualExclusiveRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = RoleRelationService::new(state.db.clone());
    service
        .check_mutual_exclusive(&req.existing_role_codes, &role_code)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        serde_json::json!({ "is_exclusive": false, "role_code": role_code }),
    )?)))
}
