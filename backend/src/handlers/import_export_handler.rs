//! 导入导出 Handler
//!
//! 提供 CSV/Excel 数据导入导出 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::import_export_service::{ExportQuery, ImportExportService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// CSV导入请求
#[derive(Debug, Deserialize)]
pub struct CsvImportRequest {
    pub import_type: String,
    pub data: String, // CSV格式的字符串
}

/// Excel导入请求
#[derive(Debug, Deserialize)]
pub struct ExcelImportRequest {
    pub import_type: String,
    pub data: Vec<Vec<String>>, // 二维数组
}

/// POST /api/v1/erp/import/csv - CSV导入
pub async fn import_csv(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CsvImportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ImportExportService::new(state.db.clone());

    // 获取导入模板
    let template = ImportExportService::get_import_template(&req.import_type)?;

    // 解析CSV数据
    let rows = ImportExportService::parse_csv(&req.data)?;

    // 验证数据
    let errors = ImportExportService::validate_import_data(&rows, &template);

    if !errors.is_empty() {
        return Ok(Json(ApiResponse::success(serde_json::json!({
            "imported": 0,
            "failed": rows.len(),
            "errors": errors,
        }))));
    }

    // 执行实际导入
    let result = service
        .import_data(&req.import_type, &rows, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "导入完成",
    )))
}

/// POST /api/v1/erp/import/excel - Excel导入
pub async fn import_excel(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExcelImportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ImportExportService::new(state.db.clone());

    // 获取导入模板
    let template = ImportExportService::get_import_template(&req.import_type)?;

    // 验证数据
    let errors = ImportExportService::validate_import_data(&req.data, &template);

    if !errors.is_empty() {
        return Ok(Json(ApiResponse::success(serde_json::json!({
            "imported": 0,
            "failed": req.data.len(),
            "errors": errors,
        }))));
    }

    // 执行实际导入
    let result = service
        .import_data(&req.import_type, &req.data, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "导入完成",
    )))
}

/// GET /api/v1/erp/import/templates/:import_type - 下载导入模板
pub async fn download_template(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(import_type): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let template = ImportExportService::get_import_template(&import_type)?;

    // 生成CSV模板内容
    let headers: Vec<String> = template.columns.iter().map(|c| c.title.clone()).collect();
    let example_row: Vec<String> = template
        .columns
        .iter()
        .map(|c| c.example.clone().unwrap_or_default())
        .collect();

    let csv_content = ImportExportService::generate_csv(&headers, &[example_row])?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}_template.csv", import_type),
        "content": csv_content,
        "columns": template.columns,
    }))))
}

/// GET /api/v1/erp/export/csv/:export_type - CSV导出
pub async fn export_csv(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ImportExportService::new(state.db.clone());

    // 从数据库查询数据
    let (headers, data) = service.export_data(&export_type, &query).await?;

    let csv_content = ImportExportService::generate_csv(&headers, &data)?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}.csv", export_type),
        "content": csv_content,
        "total": data.len(),
        "exported_at": chrono::Utc::now().to_rfc3339(),
    }))))
}

/// GET /api/v1/erp/export/excel/:export_type - Excel导出
pub async fn export_excel_type(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ImportExportService::new(state.db.clone());

    // 从数据库查询数据
    let (headers, data) = service.export_data(&export_type, &query).await?;

    // 生成Excel格式（使用CSV作为简化实现）
    let csv_content = ImportExportService::generate_csv(&headers, &data)?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}.xlsx", export_type),
        "content": csv_content,
        "content_type": "text/csv", // 简化实现使用CSV格式
        "total": data.len(),
        "exported_at": chrono::Utc::now().to_rfc3339(),
    }))))
}
