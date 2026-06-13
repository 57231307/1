
use crate::middleware::auth_context::AuthContext;
use crate::models::quality_standard;
use crate::services::quality_standard_service::QualityStandardService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
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
pub struct CreateQualityStandardRequest {
    /// 标准编码
    pub standard_code: Option<String>,
    /// 标准名称
    pub standard_name: String,
    /// 标准类型：product（产品标准）或 process（工艺标准）
    pub standard_type: Option<String>,
    /// 版本号
    pub version: Option<String>,
    /// 标准内容
    pub content: Option<String>,
    /// 生效日期，格式：YYYY-MM-DD
    pub effective_date: Option<String>,
    /// 失效日期，格式：YYYY-MM-DD（可选）
    pub expiry_date: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

/// 更新质量标准请求 DTO
#[derive(Debug, Deserialize)]
pub struct UpdateQualityStandardRequest {
    /// 标准名称
    pub standard_name: Option<String>,
    /// 标准类型
    pub standard_type: Option<String>,
    /// 标准内容
    pub content: Option<String>,
    /// 状态：draft, approved, published, rejected
    pub status: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

/// 创建版本历史请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateVersionHistoryRequest {
    /// 标准ID
    pub standard_id: i32,
    /// 新版本号
    pub version: String,
    /// 变更原因
    pub change_reason: String,
    /// 变更内容
    pub change_content: String,
}

/// 质量标准审批请求 DTO
#[derive(Debug, Deserialize)]
pub struct QualityApproveRequest {
    /// 审批意见
    pub approval_comment: Option<String>,
}

/// 获取质量标准列表
pub async fn list_standards(
    Query(params): Query<QualityStandardQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_standard::Model>>>, AppError> {
    info!("用户 {} 正在查询质量标准列表", auth.username);

    let service = QualityStandardService::new(state.db.clone());
    let query_params = crate::services::quality_standard_service::QualityStandardQueryParams {
        standard_type: params.standard_type,
        status: params.status,
        page: params.page.unwrap_or_default(),
        page_size: params.page_size.unwrap_or(10),
    };

    let (standards, _total) = service.get_standards_list(query_params).await?;
    info!("质量标准列表查询成功，共 {} 条记录", standards.len());

    Ok(Json(ApiResponse::success(standards)))
}

/// 获取质量标准详情
pub async fn get_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!("用户 {} 正在查询质量标准详情：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    let standard = service.get_standard_by_id(id).await?;

    info!("质量标准详情查询成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
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
        auth.username,
        req.standard_code.as_deref().unwrap_or("自动生成")
    );

    let service = QualityStandardService::new(state.db.clone());
    let standard = service
        .create_standard(
            crate::services::quality_standard_service::CreateQualityStandardRequest {
                standard_code: req.standard_code,
                standard_name: req.standard_name,
                standard_type: req.standard_type,
                version: req.version,
                content: req.content,
                effective_date: req
                    .effective_date
                    .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
                expiry_date: req
                    .expiry_date
                    .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("质量标准创建成功：{}", standard.standard_code);
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

    let service = QualityStandardService::new(state.db.clone());
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

    info!("质量标准更新成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}

/// 审批质量标准
#[axum::debug_handler]
pub async fn approve_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<QualityApproveRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审批质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service
        .approve_standard(id, auth.user_id, req.approval_comment)
        .await?;

    info!("质量标准审批成功：{}", id);
    Ok(Json(ApiResponse::success("审批成功".to_string())))
}

/// 发布质量标准
#[axum::debug_handler]
pub async fn publish_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在发布质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service.publish_standard(id, auth.user_id).await?;

    info!("质量标准发布成功：{}", id);
    Ok(Json(ApiResponse::success("发布成功".to_string())))
}

/// GET /api/v1/erp/quality-standards/:id/versions - 获取质量标准版本历史
pub async fn list_versions(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    info!("用户 {} 查询质量标准 {} 的版本历史", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    let versions = service.get_version_history(id).await?;

    let version_list: Vec<serde_json::Value> = versions
        .into_iter()
        .map(|v| serde_json::to_value(v).unwrap_or_default())
        .collect();

    Ok(Json(ApiResponse::success(version_list)))
}

/// POST /api/v1/erp/quality-standards/versions - 创建版本历史
#[axum::debug_handler]
pub async fn create_version_history(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateVersionHistoryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 为质量标准 {} 创建新版本",
        auth.username, req.standard_id
    );

    let service = QualityStandardService::new(state.db.clone());

    let create_req = crate::services::quality_standard_service::CreateVersionHistoryRequest {
        standard_id: req.standard_id,
        version: req.version,
        change_reason: req.change_reason,
        change_content: req.change_content,
    };

    let version = service
        .create_version_history(create_req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(version)?,
        "版本历史创建成功",
    )))
}

/// DELETE /api/v1/erp/quality-standards/:id - 删除质量标准
#[axum::debug_handler]
pub async fn delete_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除质量标准 {}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service.delete_standard(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "质量标准已删除",
    )))
}
