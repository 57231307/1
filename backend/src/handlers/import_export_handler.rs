//! 导入导出 Handler
//!
//! 提供 CSV/Excel 数据导入导出 API 接口
//!
//! 安全说明（漏洞 #8 修复）：
//! - CSV / Excel 导入端点对请求体大小有限制（详见 import_export_service::MAX_CSV_BYTES /
//!   MAX_EXCEL_ROWS / MAX_EXCEL_COLS / MAX_CELL_LEN），防止已认证用户发送超大请求触发
//!   OOM DoS / 数据库压力 / 服务崩溃。
//! - 校验层次：DTO #[validate] → handler 早期校验（友好提示）→ service 层 defense-in-depth。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::import_export_service::{
    ExportQuery, ImportExportService, ImportResult,
};
use crate::utils::error::AppError;
use crate::utils::export_concurrency::ExportConcurrencyGuard;
use crate::utils::response::ApiResponse;

/// Excel 导入请求（data 行数 validator 校验上限 1 万行）。
/// 单元格/列数限制由 handler 入口 + service 层 defense-in-depth 双重把关。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct ExcelImportRequest {
    pub import_type: String,
    #[validate(length(max = 10_000, message = "Excel 数据超过 1 万行上限"))]
    pub data: Vec<Vec<String>>, // 二维数组
}

/// POST /api/v1/erp/import/excel - Excel导入
pub async fn import_excel(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExcelImportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 安全漏洞 #8 修复：DTO 校验失败（行数超过 1 万行）→ 友好错误
    req.validate()?;
    // handler 入口早期校验行列数与单元格长度
    validate_excel_data(&req.data)?;

    let service = ImportExportService::new(state.db.clone());
    let template = ImportExportService::get_import_template(&req.import_type)?;

    // 批次 127 v8 复审 P2 修复：导入前创建任务记录（status=running）
    let task_id = service
        .create_import_task(&req.import_type, req.data.len() as u64, auth.user_id)
        .await?;

    let errors = ImportExportService::validate_import_data(&req.data, &template);
    if !errors.is_empty() {
        let fail_result = ImportResult {
            imported: 0,
            failed: req.data.len() as u64,
            errors,
        };
        return finish_import_validation_failure(&service, task_id, fail_result).await;
    }

    // 执行实际导入
    let result = service
        .import_data(&req.import_type, &req.data, auth.user_id)
        .await?;

    // 导入完成：更新任务记录
    if let Err(e) = service.update_import_task(task_id, &result).await {
        tracing::warn!(error = %e, task_id, "更新导入任务记录为完成状态失败");
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "导入完成",
    )))
}

/// 校验 Excel 数据行列数与单元格长度上限
fn validate_excel_data(data: &[Vec<String>]) -> Result<(), AppError> {
    use crate::services::import_export_service::{MAX_CELL_LEN, MAX_EXCEL_COLS, MAX_EXCEL_ROWS};

    if data.len() > MAX_EXCEL_ROWS {
        return Err(AppError::validation(format!(
            "Excel 数据超过 {} 行上限：当前 {} 行",
            MAX_EXCEL_ROWS,
            data.len()
        )));
    }
    for (row_idx, row) in data.iter().enumerate() {
        if row.len() > MAX_EXCEL_COLS {
            return Err(AppError::validation(format!(
                "Excel 第 {} 行列数超过 {} 列上限：当前 {} 列",
                row_idx + 1,
                MAX_EXCEL_COLS,
                row.len()
            )));
        }
        for (col_idx, cell) in row.iter().enumerate() {
            if cell.len() > MAX_CELL_LEN {
                return Err(AppError::validation(format!(
                    "Excel 第 {} 行第 {} 列单元格超过 {} 字符上限：当前 {} 字符",
                    row_idx + 1,
                    col_idx + 1,
                    MAX_CELL_LEN,
                    cell.len()
                )));
            }
        }
    }
    Ok(())
}

/// 验证失败时更新任务为 failed 并返回失败结果
async fn finish_import_validation_failure(
    service: &ImportExportService,
    task_id: i32,
    fail_result: ImportResult,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if let Err(e) = service.update_import_task(task_id, &fail_result).await {
        tracing::warn!(error = %e, task_id, "更新导入任务记录为 failed 状态失败");
    }
    Ok(Json(ApiResponse::success(serde_json::to_value(
        fail_result,
    )?)))
}

/// GET /api/v1/erp/import/templates/:import_type - 下载导入模板
pub async fn download_template(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(import_type): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let template = ImportExportService::get_import_template(&import_type)?;

    // 生成模板表头与示例行
    let headers: Vec<String> = template.columns.iter().map(|c| c.title.clone()).collect();
    let example_row: Vec<String> = template
        .columns
        .iter()
        .map(|c| c.example.clone().unwrap_or_default())
        .collect();

    // 规则 3：模板导出统一使用 xlsx 格式
    let xlsx_bytes = ImportExportService::generate_xlsx(&headers, &[example_row])?;
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD.encode(&xlsx_bytes);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}_template.xlsx", import_type),
        "content": content,
        "content_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "columns": template.columns,
    }))))
}

/// GET /api/v1/erp/export/xlsx/:export_type - 数据导出（xlsx）
pub async fn export_xlsx(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // V15 P1-9-1：全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let service = ImportExportService::new(state.db.clone());

    let (headers, data) = service.export_data(&export_type, &query).await?;

    // 规则 3：导出统一使用 xlsx 格式
    let xlsx_bytes = ImportExportService::generate_xlsx(&headers, &data)?;
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD.encode(&xlsx_bytes);

    // 审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some(export_type.clone()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", export_type)),
        description: Some(format!(
            "用户 {} 导出 {} 数据为 xlsx（共 {} 条）",
            auth.username,
            export_type,
            data.len()
        )),
        request_method: Some("GET".to_string()),
        request_path: Some(format!("/api/v1/erp/export/xlsx/{}", export_type)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "export_type": export_type,
            "format": "xlsx",
            "total": data.len(),
            "status_filter": query.status,
            "date_from": query.date_from,
            "date_to": query.date_to,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}.xlsx", export_type),
        "content": content,
        "content_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "total": data.len(),
        "exported_at": chrono::Utc::now().to_rfc3339(),
    }))))
}

/// GET /api/v1/erp/export/excel/:export_type - Excel导出（xlsx）
pub async fn export_excel_type(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // V15 P1-9-1：全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let service = ImportExportService::new(state.db.clone());

    let (headers, data) = service.export_data(&export_type, &query).await?;

    // 规则 3：导出统一使用 xlsx 格式
    let xlsx_bytes = ImportExportService::generate_xlsx(&headers, &data)?;
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD.encode(&xlsx_bytes);

    // P1 8-6 修复：export_excel_type 补审计日志（原仅 tracing::info）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some(export_type.clone()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", export_type)),
        description: Some(format!(
            "用户 {} 导出 {} 数据为 xlsx（共 {} 条）",
            auth.username,
            export_type,
            data.len()
        )),
        request_method: Some("GET".to_string()),
        request_path: Some(format!("/api/v1/erp/export/excel/{}", export_type)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "export_type": export_type,
            "format": "xlsx",
            "total": data.len(),
            "status_filter": query.status,
            "date_from": query.date_from,
            "date_to": query.date_to,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}.xlsx", export_type),
        "content": content,
        "content_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "total": data.len(),
        "exported_at": chrono::Utc::now().to_rfc3339(),
    }))))
}

/// B12-P2-5：流式导出端点（直接返回文件，不经过 base64 编码）
/// GET /api/v1/erp/export/stream/:export_type - 流式导出（xlsx）
pub async fn export_stream(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<axum::response::Response, AppError> {
    // V15 P1-9-1：全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let service = ImportExportService::new(state.db.clone());

    let (headers, data) = service.export_data(&export_type, &query).await?;

    // 规则 3：导出统一使用 xlsx 格式
    let xlsx_bytes = ImportExportService::generate_xlsx(&headers, &data)?;

    // 审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some(export_type.clone()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", export_type)),
        description: Some(format!(
            "用户 {} 流式导出 {} 数据为 xlsx（共 {} 条）",
            auth.username,
            export_type,
            data.len()
        )),
        request_method: Some("GET".to_string()),
        request_path: Some(format!("/api/v1/erp/export/stream/{}", export_type)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "export_type": export_type,
            "format": "xlsx",
            "total": data.len(),
            "streaming": true,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    // 直接返回文件流（不经过 base64 编码，减少内存占用）
    let response = axum::response::Response::builder()
        .header(
            "Content-Type",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}.xlsx\"", export_type),
        )
        .header("Content-Length", xlsx_bytes.len())
        .body(axum::body::Body::from(xlsx_bytes))
        .map_err(|e| AppError::internal(format!("构建响应失败: {}", e)))?;

    Ok(response)
}
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct ImportTemplateListItem {
    pub import_type: String,
    pub name: String,
    pub description: String,
}

/// 导入任务列表项
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct ImportTaskItem {
    pub id: i32,
    pub import_type: String,
    pub status: String,
    pub total_rows: u64,
    pub imported_rows: u64,
    pub failed_rows: u64,
    pub created_at: String,
}

/// GET /api/v1/erp/data-import/templates - 获取导入模板列表
pub async fn list_import_templates(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ImportTemplateListItem>>>, AppError> {
    let templates = vec![
        ImportTemplateListItem {
            import_type: "products".to_string(),
            name: "产品导入模板".to_string(),
            description: "用于批量导入产品信息".to_string(),
        },
        ImportTemplateListItem {
            import_type: "customers".to_string(),
            name: "客户导入模板".to_string(),
            description: "用于批量导入客户信息".to_string(),
        },
        ImportTemplateListItem {
            import_type: "inventory".to_string(),
            name: "库存导入模板".to_string(),
            description: "用于批量导入库存信息".to_string(),
        },
    ];
    Ok(Json(ApiResponse::success(templates)))
}

/// GET /api/v1/erp/data-import/tasks - 获取导入任务列表
pub async fn list_import_tasks(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ImportTaskItem>>>, AppError> {
    // 批次 127 v8 复审 P2 修复：原返回空列表 vec![]，现真实接入数据库查询
    let service = ImportExportService::new(state.db.clone());
    let tasks = service.list_import_tasks().await?;

    // 将 Model 映射为 ImportTaskItem DTO（i64 → u64 转换，created_at → RFC3339 字符串）
    let items = tasks
        .into_iter()
        .map(|t| ImportTaskItem {
            id: t.id,
            import_type: t.import_type,
            status: t.status,
            total_rows: t.total_rows.max(0) as u64,
            imported_rows: t.imported_rows.max(0) as u64,
            failed_rows: t.failed_rows.max(0) as u64,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}
