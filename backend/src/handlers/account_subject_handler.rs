//! 会计科目 Handler
//!
//! HTTP 接口层

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::middleware::auth_context::AuthContext;
use crate::models::account_subject;
use crate::services::account_subject_service::{
    AccountSubjectService, CreateSubjectRequest, SubjectQueryParams, UpdateSubjectRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct SubjectQuery {
    pub level: Option<i32>,
    pub parent_id: Option<i32>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// 创建请求
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSubjectRequestDto {
    pub code: String,
    pub name: String,
    pub level: i32,
    #[serde(default)]
    pub parent_id: Option<i32>,
    #[serde(default)]
    pub balance_direction: Option<String>,
    #[serde(default)]
    pub assist_customer: bool,
    #[serde(default)]
    pub assist_supplier: bool,
    #[serde(default)]
    pub assist_batch: bool,
    #[serde(default)]
    pub assist_color_no: bool,
    #[serde(default)]
    pub enable_dual_unit: bool,
}

/// 更新请求
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubjectRequestDto {
    pub name: Option<String>,
    pub balance_direction: Option<String>,
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub enable_dual_unit: bool,
}

/// 查询科目列表
#[axum::debug_handler]
pub async fn list_subjects(
    Query(params): Query<SubjectQuery>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<account_subject::Model>>>, AppError> {
    info!("用户 {} 查询会计科目列表", auth.username);

    let service = AccountSubjectService::new(db);
    let query_params = SubjectQueryParams {
        level: params.level,
        parent_id: params.parent_id,
        status: params.status,
        keyword: params.keyword,
    };

    let subjects = service
        .get_list(query_params)
        .await?;
    info!(
        "用户 {} 查询会计科目成功，共 {} 条",
        auth.username,
        subjects.len()
    );

    Ok(Json(ApiResponse::success(subjects)))
}

/// 查询单个科目
pub async fn get_subject(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<account_subject::Model>>, AppError> {
    info!("用户 {} 查询会计科目 ID: {}", auth.username, id);

    let service = AccountSubjectService::new(db);
    let subject = service.get_by_id(id).await?;
    info!("用户 {} 查询会计科目成功", auth.username);

    Ok(Json(ApiResponse::success(subject)))
}

/// 查询科目树
pub async fn get_subject_tree(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<
    Json<ApiResponse<Vec<crate::services::account_subject_service::SubjectTreeNode>>>,
    AppError,
> {
    info!("用户 {} 查询会计科目树", auth.username);

    let service = AccountSubjectService::new(db);
    let tree = service.get_tree().await?;
    info!("用户 {} 查询会计科目树成功", auth.username);

    Ok(Json(ApiResponse::success(tree)))
}

/// 创建科目
#[axum::debug_handler]
pub async fn create_subject(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<CreateSubjectRequestDto>,
) -> Result<Json<ApiResponse<account_subject::Model>>, AppError> {
    info!("用户 {} 创建会计科目：{}", auth.username, req.code);

    let service = AccountSubjectService::new(db);
    let create_req = CreateSubjectRequest {
        code: req.code,
        name: req.name,
        level: req.level,
        parent_id: req.parent_id,
        balance_direction: req.balance_direction,
        assist_customer: req.assist_customer,
        assist_supplier: req.assist_supplier,
        assist_batch: req.assist_batch,
        assist_color_no: req.assist_color_no,
        enable_dual_unit: req.enable_dual_unit,
    };

    let subject = service.create(create_req, auth.user_id).await?;
    info!("用户 {} 创建会计科目成功：{}", auth.username, subject.code);

    Ok(Json(ApiResponse::success_with_message(
        subject,
        "会计科目创建成功",
    )))
}

/// 更新科目
#[axum::debug_handler]
pub async fn update_subject(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<UpdateSubjectRequestDto>,
) -> Result<Json<ApiResponse<account_subject::Model>>, AppError> {
    info!("用户 {} 更新会计科目 ID: {}", auth.username, id);

    let service = AccountSubjectService::new(db);
    let update_req = UpdateSubjectRequest {
        name: req.name,
        balance_direction: req.balance_direction,
        assist_customer: req.assist_customer,
        assist_supplier: req.assist_supplier,
        assist_batch: req.assist_batch,
        assist_color_no: req.assist_color_no,
        enable_dual_unit: req.enable_dual_unit,
    };

    let subject = service
        .update(id, update_req, auth.user_id)
        .await?;
    info!("用户 {} 更新会计科目成功：{}", auth.username, subject.code);

    Ok(Json(ApiResponse::success_with_message(
        subject,
        "会计科目更新成功",
    )))
}

/// 删除科目
pub async fn delete_subject(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除会计科目 ID: {}", auth.username, id);

    let service = AccountSubjectService::new(db);
    service.delete(id).await?;
    info!("用户 {} 删除会计科目成功", auth.username);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "会计科目删除成功",
    )))
}
