use crate::middleware::auth_context::AuthContext;
use crate::models::quality_standard;
use crate::services::quality_standard_service::QualityStandardService;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

/// 质量标准查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct QualityStandardQuery {
    pub standard_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建质量标准请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateQualityStandardRequest {
    pub standard_code: String,
    pub standard_name: String,
    pub standard_type: String,
    pub version: String,
    pub content: String,
    pub effective_date: String,
    pub expiry_date: Option<String>,
    pub remark: Option<String>,
}

/// 更新质量标准请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateQualityStandardRequest {
    pub standard_name: Option<String>,
    pub standard_type: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建版本历史请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CreateVersionHistoryRequest {
    pub standard_id: i32,
    pub version: String,
    pub change_reason: String,
    pub change_content: String,
}

/// 质量标准审批请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityApproveRequest {
    pub approval_comment: Option<String>,
}

/// 获取质量标准列表
pub async fn list_standards(
    Query(params): Query<QualityStandardQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_standard::Model>>>, AppError> {
    info!("用户 {} 正在查询质量标准列表", auth.username);

    let service = QualityStandardService::new(db);
    let query_params = crate::services::quality_standard_service::QualityStandardQueryParams {
        standard_type: params.standard_type,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (standards, _total) = service.get_standards_list(query_params).await?;
    info!("质量标准列表查询成功，共 {} 条记录", standards.len());

    Ok(Json(ApiResponse::success(standards)))
}

/// 创建质量标准
#[axum::debug_handler]
pub async fn create_standard(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualityStandardRequest>,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!(
        "用户 {} 正在创建质量标准：{}",
        auth.username, req.standard_code
    );

    let effective_date = NaiveDate::parse_from_str(&req.effective_date, "%Y-%m-%d")
        .map_err(|e| AppError::ValidationError(format!("日期格式错误：{}", e)))?;
    let expiry_date = req
        .expiry_date
        .map(|d| {
            NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                .map_err(|e| AppError::ValidationError(format!("日期格式错误：{}", e)))
        })
        .transpose()?;

    let service = QualityStandardService::new(db);
    let standard = service
        .create_standard(
            crate::services::quality_standard_service::CreateQualityStandardRequest {
                standard_code: req.standard_code,
                standard_name: req.standard_name,
                standard_type: req.standard_type,
                version: req.version,
                content: req.content,
                effective_date,
                expiry_date,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("质量标准创建成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}

/// 获取质量标准详情
pub async fn get_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!("用户 {} 正在查询质量标准详情：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    let standard = service.get_standard_by_id(id).await?;

    info!("质量标准详情查询成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}

/// 更新质量标准
#[axum::debug_handler]
pub async fn update_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateQualityStandardRequest>,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!("用户 {} 正在更新质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    let standard = service
        .update_standard(
            id,
            crate::services::quality_standard_service::UpdateQualityStandardRequest {
                standard_name: req.standard_name,
                standard_type: req.standard_type,
                content: req.content,
                status: req.status,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("质量标准更新成功：{}", id);
    Ok(Json(ApiResponse::success(standard)))
}

/// 删除质量标准
pub async fn delete_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    service.delete_standard(id, auth.user_id).await?;

    info!("质量标准删除成功：{}", id);
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 获取版本历史列表
pub async fn list_versions(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_standard::Model>>>, AppError> {
    info!("用户 {} 正在查询质量标准版本历史：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    let versions = service.get_version_history(id).await?;

    info!("质量标准版本历史查询成功，共 {} 个版本", versions.len());
    Ok(Json(ApiResponse::success(versions)))
}

/// 质量标准审批
#[axum::debug_handler]
pub async fn approve_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<QualityApproveRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审批质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    service
        .approve_standard(id, auth.user_id, req.approval_comment)
        .await?;

    info!("质量标准审批通过：{}", id);
    Ok(Json(ApiResponse::success("审批通过".to_string())))
}

/// 质量标准发布
#[axum::debug_handler]
pub async fn publish_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在发布质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    service.publish_standard(id, auth.user_id).await?;

    info!("质量标准发布成功：{}", id);
    Ok(Json(ApiResponse::success("发布成功".to_string())))
}

/// 创建版本历史（版本升级）
#[allow(dead_code)]
pub async fn create_version_history(
    Path(id): Path<i32>,
    Json(req): Json<CreateVersionHistoryRequest>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!("用户 {} 正在创建质量标准版本历史：{}", auth.username, id);

    let service = QualityStandardService::new(db);
    let standard = service
        .create_version_history(
            crate::services::quality_standard_service::CreateVersionHistoryRequest {
                standard_id: id,
                version: req.version,
                change_reason: req.change_reason,
                change_content: req.change_content,
            },
            auth.user_id,
        )
        .await?;

    info!("质量标准版本历史创建成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}
