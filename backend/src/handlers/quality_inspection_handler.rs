use crate::middleware::auth_context::AuthContext;
use crate::models::quality_inspection;
use crate::models::quality_inspection_record;
use crate::models::unqualified_product;
use crate::services::quality_inspection_service::{
    CreateInspectionRecordRequest, CreateQualityInspectionStandardRequest,
    ProcessUnqualifiedRequest, QualityInspectionService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct QualityInspectionQuery {
    pub inspection_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RecordQuery {
    pub product_id: Option<i32>,
    pub batch_number: Option<String>,
    pub inspection_result: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DefectQuery {
    pub record_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InspectionRecordResponse {
    pub id: i32,
    pub record_number: String,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub batch_number: String,
    pub color_code: Option<String>,
    pub quantity: i32,
    pub qualified_quantity: i32,
    pub unqualified_quantity: i32,
    pub inspection_date: String,
    pub inspector_id: i32,
    pub inspector_name: Option<String>,
    pub inspection_result: String,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DefectResponse {
    pub id: i32,
    pub record_id: i32,
    pub defect_type: String,
    pub defect_description: String,
    pub quantity: i32,
    pub severity_level: String,
    pub handling_method: String,
    pub handler_id: Option<i32>,
    pub handler_name: Option<String>,
    pub handling_date: Option<String>,
    pub status: String,
}

pub async fn list_standards(
    Query(params): Query<QualityInspectionQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_inspection::Model>>>, AppError> {
    info!("用户 {} 正在查询质量检验标准列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: params.inspection_type,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (standards, _total) = service.get_standards_list(query_params).await?;
    info!("质量检验标准列表查询成功，共 {} 条记录", standards.len());

    Ok(Json(ApiResponse::success(standards)))
}

pub async fn create_standard(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualityInspectionStandardRequest>,
) -> Result<Json<ApiResponse<quality_inspection::Model>>, AppError> {
    info!("用户 {} 正在创建质量检验标准", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let standard = service.create_standard(req, auth.user_id).await?;
    info!("质量检验标准创建成功，ID：{}", standard.id);

    Ok(Json(ApiResponse::success(standard)))
}

pub async fn list_records(
    Query(params): Query<RecordQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_inspection_record::Model>>>, AppError> {
    info!("用户 {} 正在查询质量检验记录列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: params.inspection_result,
        status: None,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (records, _total) = service.get_records_list(query_params).await?;
    info!("质量检验记录列表查询成功，共 {} 条记录", records.len());

    Ok(Json(ApiResponse::success(records)))
}

#[axum::debug_handler]
pub async fn create_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateInspectionRecordRequest>,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    info!("用户 {} 正在创建质量检验记录", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let record = service.create_record(req, auth.user_id).await?;
    info!("质量检验记录创建成功，ID：{}", record.id);

    Ok(Json(ApiResponse::success(record)))
}

pub async fn get_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    info!("用户 {} 正在查询质量检验记录，ID: {}", auth.user_id, id);

    let service = QualityInspectionService::new(state.db.clone());
    let record = service.get_record_by_id(id).await?;
    info!("质量检验记录查询成功，ID：{}", record.id);

    Ok(Json(ApiResponse::success(record)))
}

pub async fn list_defects(
    Query(params): Query<DefectQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<unqualified_product::Model>>>, AppError> {
    info!("用户 {} 正在查询质量缺陷列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: None,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (defects, _total) = service.get_defects_list(query_params).await?;
    info!("质量缺陷列表查询成功，共 {} 条记录", defects.len());

    Ok(Json(ApiResponse::success(defects)))
}

#[axum::debug_handler]
pub async fn process_defect(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<ProcessUnqualifiedRequest>,
) -> Result<Json<ApiResponse<unqualified_product::Model>>, AppError> {
    info!("用户 {} 正在处理质量缺陷，记录ID: {}", auth.user_id, id);

    let service = QualityInspectionService::new(state.db.clone());
    let result = service.process_unqualified(id, req, auth.user_id).await?;
    info!("质量缺陷处理成功，ID：{}", result.id);

    Ok(Json(ApiResponse::success(result)))
}
