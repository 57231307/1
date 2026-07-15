//! 化验室打样 Handler
//!
//! v14 批次 423B：化验室打样流程贯通
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环 + §11.1.1 染色技术卡
//! 真实业务流程：打样通知单 → 打样（ABCD 多版样）→ 色样确认（OK 样）→ 复样 → 建数据库

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::{lab_dip_request, lab_dip_resample, lab_dip_sample};
use crate::services::lab_dip_service::{
    CreateLabDipRequestRequest, CreateLabDipSampleRequest, CreateResampleRequest,
    IssueTechCardRequest, LabDipRequestService, LabDipResampleService, LabDipSampleService,
    RecordMatchingResultRequest, RecordResampleResultRequest, UpdateLabDipRequestRequest,
    UpdateLabDipSampleRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 辅助函数
// ============================================================================

fn request_service(state: &AppState) -> LabDipRequestService {
    LabDipRequestService::new(state.db.clone())
}

fn sample_service(state: &AppState) -> LabDipSampleService {
    LabDipSampleService::new(state.db.clone())
}

fn resample_service(state: &AppState) -> LabDipResampleService {
    LabDipResampleService::new(state.db.clone())
}

// ============================================================================
// 查询参数
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LabDipRequestListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub request_no: Option<String>,
    pub customer_id: Option<i32>,
    pub status: Option<String>,
}

// ============================================================================
// 打样通知单 Handler
// ============================================================================

/// GET /api/v1/erp/lab-dip/requests - 分页查询打样通知单
pub async fn list_requests(
    State(state): State<AppState>,
    Query(query): Query<LabDipRequestListQuery>,
) -> Result<Json<ApiResponse<crate::utils::response::PaginatedResponse<lab_dip_request::Model>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let svc_query = crate::services::lab_dip_service::LabDipRequestQuery {
        request_no: query.request_no,
        customer_id: query.customer_id,
        status: query.status,
        page,
        page_size,
    };

    let (items, total) = request_service(&state).list(svc_query).await?;
    Ok(Json(ApiResponse::success_paginated(items, total, page, page_size)))
}

/// GET /api/v1/erp/lab-dip/requests/:id - 查询打样通知单详情
pub async fn get_request(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let req = request_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(req)))
}

/// POST /api/v1/erp/lab-dip/requests - 创建打样通知单
pub async fn create_request(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateLabDipRequestRequest>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let created = request_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(created, "打样通知单创建成功")))
}

/// PUT /api/v1/erp/lab-dip/requests/:id - 更新打样通知单
pub async fn update_request(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLabDipRequestRequest>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "打样通知单更新成功")))
}

/// DELETE /api/v1/erp/lab-dip/requests/:id - 软删除打样通知单
pub async fn delete_request(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    request_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message((), "打样通知单删除成功")))
}

/// POST /api/v1/erp/lab-dip/requests/:id/start-sampling - 开始打样（pending → sampling）
pub async fn start_sampling(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state).start_sampling(id).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "已开始打样")))
}

/// POST /api/v1/erp/lab-dip/requests/:id/submit - 送客户确认（sampling → submitted）
pub async fn submit_to_customer(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state).submit_to_customer(id).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "已送客户确认")))
}

/// 客户确认 OK 样请求体
#[derive(Debug, Deserialize)]
pub struct ApproveOkSampleRequest {
    pub sample_id: i32,
    pub comment: Option<String>,
}

/// POST /api/v1/erp/lab-dip/requests/:id/approve - 客户确认 OK 样（submitted → approved）
pub async fn approve_ok_sample(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveOkSampleRequest>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state)
        .approve_ok_sample(id, req.sample_id, req.comment)
        .await?;
    Ok(Json(ApiResponse::success_with_message(updated, "OK 样确认成功")))
}

/// 客户要求重打请求体
#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub comment: String,
}

/// POST /api/v1/erp/lab-dip/requests/:id/reject - 客户要求重打（submitted → rejected）
pub async fn reject_and_redo(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<RejectRequest>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state)
        .reject_and_redo(id, req.comment)
        .await?;
    Ok(Json(ApiResponse::success_with_message(updated, "已标记需重打")))
}

/// POST /api/v1/erp/lab-dip/requests/:id/restart - 重新打样（rejected → sampling）
pub async fn restart_sampling(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state).restart_sampling(id).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "已重新开始打样")))
}

/// 完成建库请求体
#[derive(Debug, Deserialize)]
pub struct CompleteRequest {
    pub production_recipe_id: i32,
}

/// POST /api/v1/erp/lab-dip/requests/:id/complete - 完成建库（approved → completed）
pub async fn complete_request(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<CompleteRequest>,
) -> Result<Json<ApiResponse<lab_dip_request::Model>>, AppError> {
    let updated = request_service(&state)
        .complete(id, req.production_recipe_id)
        .await?;
    Ok(Json(ApiResponse::success_with_message(updated, "已建库完成")))
}

// ============================================================================
// 打样小样 Handler
// ============================================================================

/// GET /api/v1/erp/lab-dip/samples/by-request/:request_id - 按通知单查询所有小样
pub async fn list_samples_by_request(
    State(state): State<AppState>,
    Path(request_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<lab_dip_sample::Model>>>, AppError> {
    let samples = sample_service(&state).list_by_request(request_id).await?;
    Ok(Json(ApiResponse::success(samples)))
}

/// GET /api/v1/erp/lab-dip/samples/:id - 查询小样详情
pub async fn get_sample(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<lab_dip_sample::Model>>, AppError> {
    let sample = sample_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(sample)))
}

/// POST /api/v1/erp/lab-dip/samples - 创建打样小样（ABCD 多版样）
pub async fn create_sample(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateLabDipSampleRequest>,
) -> Result<Json<ApiResponse<lab_dip_sample::Model>>, AppError> {
    let created = sample_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(created, "打样小样创建成功")))
}

/// PUT /api/v1/erp/lab-dip/samples/:id - 更新打样小样
pub async fn update_sample(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLabDipSampleRequest>,
) -> Result<Json<ApiResponse<lab_dip_sample::Model>>, AppError> {
    let updated = sample_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "打样小样更新成功")))
}

/// DELETE /api/v1/erp/lab-dip/samples/:id - 软删除小样
pub async fn delete_sample(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    sample_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success_with_message((), "打样小样删除成功")))
}

/// POST /api/v1/erp/lab-dip/samples/:id/matching - 记录对色结果
///
/// 真实业务：色差 4-5 级为 matched（OK），<4 级为 not_matched（重打）
pub async fn record_matching_result(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<RecordMatchingResultRequest>,
) -> Result<Json<ApiResponse<lab_dip_sample::Model>>, AppError> {
    let updated = sample_service(&state).record_matching_result(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "对色结果记录成功")))
}

// ============================================================================
// 复样记录 Handler
// ============================================================================

/// GET /api/v1/erp/lab-dip/resamples/by-request/:request_id - 按通知单查询复样记录
pub async fn list_resamples_by_request(
    State(state): State<AppState>,
    Path(request_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<lab_dip_resample::Model>>>, AppError> {
    let items = resample_service(&state).list_by_request(request_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/erp/lab-dip/resamples/:id - 查询复样记录详情
pub async fn get_resample(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<lab_dip_resample::Model>>, AppError> {
    let item = resample_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/v1/erp/lab-dip/resamples - 创建复样记录
///
/// 真实业务：OK 样确认后，大货生产前必须复样（用车间半制品布+生产染化料模拟大生产）
pub async fn create_resample(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateResampleRequest>,
) -> Result<Json<ApiResponse<lab_dip_resample::Model>>, AppError> {
    let created = resample_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success_with_message(created, "复样记录创建成功")))
}

/// POST /api/v1/erp/lab-dip/resamples/:id/result - 记录复样结果
///
/// 真实业务：色差 4-5 级方可投产（passed），<4 级为 failed
pub async fn record_resample_result(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<RecordResampleResultRequest>,
) -> Result<Json<ApiResponse<lab_dip_resample::Model>>, AppError> {
    let updated = resample_service(&state).record_result(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "复样结果记录成功")))
}

/// POST /api/v1/erp/lab-dip/resamples/:id/tech-card - 开具染色技术卡
///
/// 真实业务：复样通过后由研发组长开染色技术卡，附配方表+核可样+复色样
pub async fn issue_tech_card(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<IssueTechCardRequest>,
) -> Result<Json<ApiResponse<lab_dip_resample::Model>>, AppError> {
    let updated = resample_service(&state).issue_tech_card(id, req).await?;
    Ok(Json(ApiResponse::success_with_message(updated, "染色技术卡开具成功")))
}
