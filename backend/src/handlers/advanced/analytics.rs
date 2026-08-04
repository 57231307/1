//! 报表分析 handler
//!
//! 提供报表模板查询、报表执行与导出能力。
//! 适配重构后的 `services::report` 模块 API。

use std::sync::Arc;

use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::report::{ExecuteReportRequest, ReportEngineService, ReportFilter};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 报表相关端点 - 使用真实报表引擎
// ============================================================================

/// 报表模板列表
pub async fn list_report_templates(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportEngineService::new(state.db);
    let templates = service.get_predefined_templates();

    let items: Vec<ReportTemplateDto> = templates
        .into_iter()
        .map(|t| ReportTemplateDto {
            template_name: t.name,
            template_code: t.id,
            category: t.category,
            description: format!(
                "包含 {} 个列: {}",
                t.columns.len(),
                t.columns
                    .iter()
                    .map(|c| c.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            created_at: Utc::now().format("%Y-%m-%d").to_string(),
        })
        .collect();

    Ok(Json(ApiResponse::success(serde_json::to_value(items)?)))
}

#[derive(Debug, Serialize)]
struct ReportTemplateDto {
    pub template_name: String,
    pub template_code: String,
    pub category: String,
    pub description: String,
    pub created_at: String,
}

/// 执行报表
pub async fn execute_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<ReportExecuteRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportEngineService::new(state.db);

    let req = ExecuteReportRequest {
        template_id: payload.template_code,
        filters: Vec::<ReportFilter>::new(),
        parameters: None,
        date_range: None,
        format: "json".to_string(),
        use_cache: Some(true),
    };

    let report_data = service.execute_report(req).await?;

    let columns_json: Vec<serde_json::Value> = report_data
        .columns
        .iter()
        .map(|c| serde_json::json!({"key": c.key, "label": c.label}))
        .collect();

    // rows 是 Vec<serde_json::Value>，统一转为字符串映射
    let rows: Vec<HashMap<String, String>> = report_data
        .rows
        .into_iter()
        .map(|row| {
            let mut map: HashMap<String, String> = HashMap::new();
            if let serde_json::Value::Object(obj) = row {
                for (k, v) in obj.into_iter() {
                    let s = match v {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    };
                    map.insert(k, s);
                }
            }
            map
        })
        .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "columns": columns_json,
        "data": rows,
        "total_count": report_data.total_rows,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct ReportExecuteRequest {
    pub template_code: String,
}

/// 导出报表
pub async fn export_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<ReportExportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportEngineService::new(state.db.clone());

    let req = ExecuteReportRequest {
        template_id: payload.template_code.clone(),
        filters: Vec::<ReportFilter>::new(),
        parameters: None,
        date_range: None,
        format: payload.format.clone(),
        use_cache: Some(true),
    };

    let report_data = service.execute_report(req).await?;

    let format_str = match payload.format.as_str() {
        "csv" => "csv",
        "excel" | "xlsx" => "excel",
        "pdf" => "pdf",
        _ => "json",
    };

    let bytes = service
        .export_report(&report_data, format_str, &payload.template_code)
        .await?;

    let size_kb = bytes.len() / 1024;

    // V15 P2 B11-P2-1：导出审计日志（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("analytics_report".to_string()),
        resource_id: Some(payload.template_code.clone()),
        resource_name: Some(format!("{}.{}", payload.template_code, payload.format)),
        description: Some(format!(
            "用户 {} 导出分析报表 {}（共 {} 行）",
            auth.username, payload.template_code, report_data.total_rows
        )),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/analytics/reports/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": payload.format,
            "total": report_data.total_rows,
            "template_code": payload.template_code,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "status": "success",
        "format": payload.format,
        "size_bytes": bytes.len(),
        "size_kb": size_kb,
        "record_count": report_data.total_rows,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct ReportExportRequest {
    pub template_code: String,
    pub format: String,
}

// ============================================================================
// 销售分析
// ============================================================================
// 以下销售分析端点和类型已删除（CI 死代码清理）
// 业务接入时按需从 git 历史恢复
