
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
// V15 P0-S12/P0-S15 修复（Batch 475d）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct QualityInspectionQuery {
    pub inspection_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// V15 P0-S12 修复（Batch 475d）：派生 Clone，export_records 需要 clone 后覆盖分页参数用于全量导出
#[derive(Debug, Clone, Deserialize)]
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
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
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
) -> Result<Json<ApiResponse<crate::utils::response::PaginatedResponse<quality_inspection_record::Model>>>, AppError> {
    info!("用户 {} 正在查询质量检验记录列表", auth.user_id);

    let page = params.page.unwrap_or(1).clamp(1, 1000) as u64;
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100) as u64;
    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: params.inspection_result,
        status: None,
        page: page as i64,
        page_size: page_size as i64,
    };

    let (records, total) = service.get_records_list(query_params).await?;
    info!("质量检验记录列表查询成功，共 {} 条记录", records.len());

    // v11 批次 161 P2-5 修复：返回 PaginatedResponse（含 total），替代原先丢弃 _total 的 Vec 返回
    Ok(Json(ApiResponse::success_paginated(records, total, page, page_size)))
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
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
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

/// 质量检验记录导出表头（13 列）
fn record_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "检验编号".to_string(),
        "检验类型".to_string(),
        "产品ID".to_string(),
        "批次号".to_string(),
        "检验日期".to_string(),
        "检验员ID".to_string(),
        "总数量".to_string(),
        "已检数量".to_string(),
        "合格数量".to_string(),
        "不合格数量".to_string(),
        "检验结果".to_string(),
        "等级".to_string(),
    ]
}

/// 从质量检验记录 JSON 对象构建 xlsx 行
fn build_record_row(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let get_str = |key: &str| -> String {
        obj.get(key)
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else if v.is_string() {
                    v.as_str().unwrap_or("").to_string()
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_default()
    };
    vec![
        get_str("id"),
        get_str("inspection_no"),
        get_str("inspection_type"),
        get_str("product_id"),
        get_str("batch_no"),
        get_str("inspection_date"),
        get_str("inspector_id"),
        get_str("total_qty"),
        get_str("inspected_qty"),
        get_str("qualified_qty"),
        get_str("unqualified_qty"),
        get_str("inspection_result"),
        get_str("grade"),
    ]
}

/// 构造质量检验记录列表 xlsx 表格
fn build_records_table(records_json: Vec<serde_json::Value>) -> Result<XlsxTable, AppError> {
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(records_json.len());
    for r in records_json {
        let obj = r.as_object().ok_or_else(|| {
            AppError::internal("质量检验记录序列化失败：期望 JSON 对象")
        })?;
        rows.push(build_record_row(obj));
    }
    Ok(XlsxTable {
        sheet_name: "质量检验记录".to_string(),
        headers: record_export_headers(),
        rows,
    })
}

/// 异步记录质量检验记录导出操作（审计自身）
fn record_records_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    filename: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("quality_inspection_record".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出质量检验记录列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/quality-inspection/records/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

/// GET /api/v1/erp/quality-inspection/records/export - 导出质量检验记录列表（带水印 + 异步审计日志）
///
/// V15 P0-S12 修复（Batch 475d）：导出接入后端
/// - 注入水印（operator/exported_at/extra 含条数）
/// - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_records_list 取全量数据（page=1/page_size=10000）
/// - 与 list_records handler 行为对齐：inspection_result 映射到 service 的 inspection_type 字段
///   （service 内部把 inspection_type 过滤到 InspectionResult 列，语义保持一致）
pub async fn export_records(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<RecordQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = QualityInspectionService::new(state.db.clone());

    // V15 P0-S12 修复（Batch 475d）：导出全量数据
    // 与 list_records handler 保持一致：inspection_result 映射到 service 的 inspection_type 字段
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: query.inspection_result,
        status: None,
        page: 1,
        page_size: 10000,
    };

    let (records, _total) = service.get_records_list(query_params).await?;
    let row_count = records.len();

    // 序列化为 JSON 以统一字段处理
    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let table = build_records_table(records_json)?;
    let filename = format!(
        "quality_inspection_records_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_records_export_audit(&state, &auth, row_count, &filename);

    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("质量检验记录导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
