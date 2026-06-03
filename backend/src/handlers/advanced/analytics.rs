//! 报表分析 handler
//!
//! 提供报表模板查询、报表执行与导出能力。

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::report_engine_service::{ExportFormat, ReportEngineService};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

// ============================================================================
// 报表相关端点 - 使用真实报表引擎
// ============================================================================

/// 报表模板列表
pub async fn list_report_templates() -> impl IntoResponse {
    let templates = ReportEngineService::get_predefined_templates();

    let items: Vec<ReportTemplate> = templates
        .into_iter()
        .map(|t| {
            let category = match t.report_type {
                crate::services::report_engine_service::ReportType::Sales => "销售",
                crate::services::report_engine_service::ReportType::Purchase => "采购",
                crate::services::report_engine_service::ReportType::Inventory => "库存",
                crate::services::report_engine_service::ReportType::Financial => "财务",
                crate::services::report_engine_service::ReportType::Custom => "自定义",
            };

            ReportTemplate {
                template_name: t.name,
                template_code: t.id,
                category: category.to_string(),
                description: format!(
                    "包含 {} 个列: {}",
                    t.columns.len(),
                    t.columns
                        .iter()
                        .map(|c| c.title.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                created_at: Utc::now().format("%Y-%m-%d").to_string(),
            }
        })
        .collect();

    Json(ApiResponse::success(items))
}

/// 执行报表
pub async fn execute_report(
    State(state): State<AppState>,
    Json(payload): Json<ReportExecuteRequest>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db);

    match service
        .execute_report(&payload.template_code, Vec::new(), 1, 100)
        .await
    {
        Ok(report_data) => {
            let columns = report_data.columns;
            let rows: Vec<HashMap<String, String>> = report_data
                .rows
                .into_iter()
                .map(|row| {
                    let mut map = HashMap::new();
                    for (i, col) in columns.iter().enumerate() {
                        if let Some(val) = row.get(i) {
                            map.insert(col.clone(), val.clone());
                        }
                    }
                    map
                })
                .collect();

            Json(ApiResponse::success(serde_json::json!({
                "columns": columns,
                "data": rows,
                "total_count": report_data.total_count,
            })))
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Json(ApiResponse::error(format!("执行报表失败: {}", e)))
        }
    }
}

/// 导出报表
pub async fn export_report(
    State(state): State<AppState>,
    Json(payload): Json<ReportExportRequest>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db);

    match service
        .execute_report(&payload.template_code, Vec::new(), 1, 1000)
        .await
    {
        Ok(report_data) => {
            let format = match payload.format.as_str() {
                "csv" => ExportFormat::CSV,
                "excel" | "xlsx" => ExportFormat::Excel,
                "pdf" => ExportFormat::PDF,
                _ => ExportFormat::JSON,
            };

            match service.export_report(&report_data, format) {
                Ok(bytes) => {
                    let size_kb = bytes.len() / 1024;
                    Json(ApiResponse::success(serde_json::json!({
                        "status": "success",
                        "format": payload.format,
                        "size_bytes": bytes.len(),
                        "size_kb": size_kb,
                        "record_count": report_data.total_count,
                    })))
                }
                Err(e) => {
                    tracing::error!("导出报表失败: {}", e);
                    Json(ApiResponse::error(format!("导出报表失败: {}", e)))
                }
            }
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Json(ApiResponse::error(format!("执行报表失败: {}", e)))
        }
    }
}

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub template_name: String,
    pub template_code: String,
    pub category: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportExecuteRequest {
    pub template_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportExportRequest {
    pub template_code: String,
    pub format: String,
}
