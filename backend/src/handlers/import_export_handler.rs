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
    extract::{Multipart, Path, Query, State},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tracing::info;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::import_task;
use crate::models::status::import_task as import_status;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::import_export_service::{ExportQuery, ImportExportService, ImportResult};
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
        .create_import_task(&req.import_type, req.data.len() as u64, auth.user_id, None, None)
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
) -> Result<Json<ApiResponse<Vec<ImportTemplateRecord>>>, AppError> {
    let records = lock_import_templates()?;
    Ok(Json(ApiResponse::success(records.clone())))
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

// ---------- 数据导入模板明细/更新/删除/下载（内存存储，预填充系统内置模板） ----------

static IMPORT_TEMPLATE_STORE: OnceLock<Mutex<Vec<ImportTemplateRecord>>> = OnceLock::new();

fn import_template_store() -> &'static Mutex<Vec<ImportTemplateRecord>> {
    IMPORT_TEMPLATE_STORE.get_or_init(|| Mutex::new(builtin_import_template_records()))
}

fn lock_import_templates() -> Result<MutexGuard<'static, Vec<ImportTemplateRecord>>, AppError> {
    import_template_store()
        .lock()
        .map_err(|_| AppError::internal("导入模板存储不可用"))
}

fn map_import_data_type(data_type: &str) -> String {
    match data_type {
        "decimal" | "number" | "int" => "number".to_string(),
        "date" => "date".to_string(),
        "boolean" => "boolean".to_string(),
        _ => "string".to_string(),
    }
}

fn builtin_import_template_records() -> Vec<ImportTemplateRecord> {
    let specs: [(&str, &str); 3] = [("products", "product"), ("customers", "customer"), ("inventory", "inventory")];
    let created_at = "2026-01-01T00:00:00Z".to_string();

    specs
        .iter()
        .filter_map(|(import_type, module)| {
            let template = ImportExportService::get_import_template(import_type).ok()?;
            let columns: Vec<ImportColumnDto> = template
                .columns
                .iter()
                .map(|c| ImportColumnDto {
                    key: c.field.clone(),
                    label: c.title.clone(),
                    column_type: map_import_data_type(&c.data_type),
                    required: c.required,
                    default_value: c.example.clone().map(serde_json::Value::String),
                    validation_rule: None,
                })
                .collect();
            let sample_row = serde_json::Value::Object(
                template
                    .columns
                    .iter()
                    .filter_map(|c| {
                        c.example
                            .as_ref()
                            .map(|ex| (c.field.clone(), serde_json::Value::String(ex.clone())))
                    })
                    .collect(),
            );
            Some(ImportTemplateRecord {
                id: 0,
                import_type: template.import_type,
                template_code: format!("IMP_{}", import_type.to_uppercase()),
                template_name: template.name,
                description: template.description,
                module: module.to_string(),
                file_format: "xlsx".to_string(),
                columns,
                sample_data: vec![sample_row],
                status: "active".to_string(),
                created_at: created_at.clone(),
                updated_at: created_at.clone(),
            })
        })
        .enumerate()
        .map(|(idx, mut record)| {
            record.id = idx as i32 + 1;
            record
        })
        .collect()
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct ImportTemplateRecord {
    pub id: i32,
    pub import_type: String,
    pub template_code: String,
    pub template_name: String,
    pub description: String,
    pub module: String,
    pub file_format: String,
    pub columns: Vec<ImportColumnDto>,
    pub sample_data: Vec<serde_json::Value>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct ImportColumnDto {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub column_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub validation_rule: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateImportTemplateRequest {
    #[validate(length(min = 1, max = 100, message = "模板名称长度须为 1-100 字符"))]
    pub template_name: Option<String>,
    #[validate(length(max = 500, message = "描述长度不得超过 500 字符"))]
    pub description: Option<String>,
    pub module: Option<String>,
    pub file_format: Option<String>,
    pub columns: Option<Vec<ImportColumnDto>>,
    pub sample_data: Option<Vec<serde_json::Value>>,
    #[validate(length(max = 20, message = "状态取值非法"))]
    pub status: Option<String>,
}

/// GET /api/v1/erp/data-import/templates/{id} - 获取导入模板详情
pub async fn get_import_template_by_id(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<ImportTemplateRecord>>, AppError> {
    let records = lock_import_templates()?;
    let record = records
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("导入模板 {} 不存在", id)))?;
    Ok(Json(ApiResponse::success(record.clone())))
}

/// PUT /api/v1/erp/data-import/templates/{id} - 更新导入模板
pub async fn update_import_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateImportTemplateRequest>,
) -> Result<Json<ApiResponse<ImportTemplateRecord>>, AppError> {
    info!("用户 {} 更新导入模板 ID: {}", auth.username, id);
    req.validate()?;

    let mut records = lock_import_templates()?;
    let record = records
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("导入模板 {} 不存在", id)))?;

    if let Some(v) = req.template_name {
        record.template_name = v;
    }
    if let Some(v) = req.description {
        record.description = v;
    }
    if let Some(v) = req.module {
        record.module = v;
    }
    if let Some(v) = req.file_format {
        record.file_format = v;
    }
    if let Some(v) = req.columns {
        record.columns = v;
    }
    if let Some(v) = req.sample_data {
        record.sample_data = v;
    }
    if let Some(v) = req.status {
        record.status = v;
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();

    let updated = record.clone();
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "导入模板更新成功",
    )))
}

/// DELETE /api/v1/erp/data-import/templates/{id} - 删除导入模板
pub async fn delete_import_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除导入模板 ID: {}", auth.username, id);

    let mut records = lock_import_templates()?;
    let before = records.len();
    records.retain(|r| r.id != id);
    if records.len() == before {
        return Err(AppError::not_found(format!("导入模板 {} 不存在", id)));
    }

    Ok(Json(ApiResponse::success_with_message(
        (),
        "导入模板删除成功",
    )))
}

/// GET|POST /api/v1/erp/data-import/templates/{id}/download - 下载导入模板（xlsx）
pub async fn download_import_template_by_id(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    let import_type = {
        let records = lock_import_templates()?;
        let record = records
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::not_found(format!("导入模板 {} 不存在", id)))?;
        record.import_type.clone()
    };

    let template = ImportExportService::get_import_template(&import_type)?;
    let headers: Vec<String> = template.columns.iter().map(|c| c.title.clone()).collect();
    let example_row: Vec<String> = template
        .columns
        .iter()
        .map(|c| c.example.clone().unwrap_or_default())
        .collect();

    let xlsx_bytes = ImportExportService::generate_xlsx(&headers, &[example_row])?;

    let response = axum::response::Response::builder()
        .header(
            "Content-Type",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}_template.xlsx\"", import_type),
        )
        .header("Content-Length", xlsx_bytes.len())
        .body(axum::body::Body::from(xlsx_bytes))
        .map_err(|e| AppError::internal(format!("构建响应失败: {}", e)))?;

    Ok(response)
}

// ============================================================================
// 数据导入任务生命周期（复用 import_tasks 表，models/import_task.rs）
// 对应前端 api/data-import.ts 的任务详情/取消/重试/错误日志调用
// ============================================================================

/// 将 import_task Model 映射为前端 `ImportTask` 期望的 JSON 结构
/// （m0054 补列后 template_id/file_name 从任务表真实落库值读取）
fn import_task_to_frontend_json(t: import_task::Model) -> serde_json::Value {
    let processed = t.imported_rows.max(0) + t.failed_rows.max(0);
    serde_json::json!({
        "id": t.id,
        "task_code": format!("IMP-{:06}", t.id),
        "template_id": t.template_id.unwrap_or(0),
        "template_name": t.import_type,
        "file_name": t.file_name.clone().unwrap_or_default(),
        "file_path": "",
        "status": t.status,
        "total_rows": t.total_rows.max(0),
        "processed_rows": processed,
        "success_rows": t.imported_rows.max(0),
        "failed_rows": t.failed_rows.max(0),
        "error_log": build_import_task_error_log(&t),
        "created_by": t.user_id.unwrap_or(0),
        "created_by_name": "",
        "created_at": t.created_at.to_rfc3339(),
        "completed_at": t.updated_at.to_rfc3339(),
    })
}

/// 生成导入任务的错误日志说明文本（任务表无独立 error_log 字段，按统计聚合生成）
fn build_import_task_error_log(t: &import_task::Model) -> String {
    if t.failed_rows <= 0 {
        String::new()
    } else {
        format!(
            "导入类型 {}：共 {} 行，其中 {} 行失败；请修正数据后使用重试端点重新导入",
            t.import_type,
            t.total_rows.max(0),
            t.failed_rows.max(0)
        )
    }
}

/// POST /api/v1/erp/data-import/tasks - 上传文件创建导入任务（对应前端 api/data-import.ts uploadImportFile）
/// multipart 字段：file（文件，最小实现仅登记文件名，不做真实解析导入）+ template_id
pub async fn create_import_task_from_upload(
    State(state): State<AppState>,
    auth: AuthContext,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 上传文件创建导入任务", auth.username);

    let mut template_id: Option<i32> = None;
    let mut file_name = String::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name() {
            Some("template_id") => {
                let text = field.text().await.unwrap_or_default();
                template_id = text.trim().parse::<i32>().ok();
            }
            Some("file") => {
                if let Some(name) = field.file_name() {
                    file_name = name.to_string();
                }
                // 最小实现：丢弃文件内容，仅记录文件名（真实解析导入走 /import/excel 端点）
            }
            _ => {}
        }
    }

    let template_id = template_id.ok_or_else(|| AppError::bad_request("缺少 template_id 字段"))?;

    // 从内存模板存储定位模板，取其 import_type 作为任务的导入类型
    let import_type = {
        let records = lock_import_templates()?;
        records
            .iter()
            .find(|r| r.id == template_id)
            .map(|r| r.import_type.clone())
            .ok_or_else(|| AppError::not_found(format!("导入模板 {} 不存在", template_id)))?
    };

    let service = ImportExportService::new(state.db.clone());
    let task_id = service
        .create_import_task(&import_type, 0, auth.user_id, Some(file_name), Some(template_id))
        .await?;

    let task = import_task::Entity::find_by_id(task_id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::internal("导入任务记录创建后查询失败"))?;

    info!(
        "用户 {} 创建导入任务成功：ID={}，模板={}，文件={}",
        auth.username, task_id, import_type, file_name
    );

    Ok(Json(ApiResponse::success_with_message(
        import_task_to_frontend_json(task),
        "导入任务已创建",
    )))
}

/// GET /api/v1/erp/data-import/tasks/{id} - 获取导入任务详情（对应前端 api/data-import.ts getImportTask）
pub async fn get_import_task_detail(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询导入任务详情 ID={}", auth.username, id);

    let task = import_task::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("导入任务 {} 不存在", id)))?;

    Ok(Json(ApiResponse::success(import_task_to_frontend_json(
        task,
    ))))
}

/// POST /api/v1/erp/data-import/tasks/{id}/cancel - 取消导入任务（对应前端 api/data-import.ts cancelImportTask）
/// 仅运行中的任务可取消，取消后任务标记为 cancelled
pub async fn cancel_import_task(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 取消导入任务 ID={}", auth.username, id);

    let task = import_task::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("导入任务 {} 不存在", id)))?;

    if task.status != import_status::RUNNING {
        return Err(AppError::business(format!(
            "导入任务 {} 当前状态为 {}，仅运行中的任务可取消",
            id, task.status
        )));
    }

    let mut active: import_task::ActiveModel = task.into();
    active.status = Set(import_status::CANCELLED.to_string());
    active.updated_at = Set(chrono::Utc::now().into());
    let updated = active.update(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::success_with_message(
        import_task_to_frontend_json(updated),
        "导入任务已取消",
    )))
}

/// POST /api/v1/erp/data-import/tasks/{id}/retry - 重试导入任务（对应前端 api/data-import.ts retryImportTask）
/// 仅失败/部分成功/已取消的任务可重试，重试后状态重置为 running
pub async fn retry_import_task(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 重试导入任务 ID={}", auth.username, id);

    let task = import_task::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("导入任务 {} 不存在", id)))?;

    if matches!(
        task.status.as_str(),
        import_status::RUNNING | import_status::SUCCESS
    ) {
        return Err(AppError::business(format!(
            "导入任务 {} 当前状态为 {}，仅失败/部分成功/已取消的任务可重试",
            id, task.status
        )));
    }

    let mut active: import_task::ActiveModel = task.into();
    active.status = Set(import_status::RUNNING.to_string());
    active.updated_at = Set(chrono::Utc::now().into());
    let updated = active.update(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::success_with_message(
        import_task_to_frontend_json(updated),
        "导入任务已重新开始",
    )))
}

/// GET /api/v1/erp/data-import/tasks/{id}/error-log - 获取导入任务错误日志（对应前端 api/data-import.ts downloadErrorLog）
/// 返回 JSON（前端以 blob 方式接收后可保存为日志文件）
pub async fn get_import_task_error_log(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 获取导入任务错误日志 ID={}", auth.username, id);

    let task = import_task::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("导入任务 {} 不存在", id)))?;

    let error_log = build_import_task_error_log(&task);
    Ok(Json(ApiResponse::success(serde_json::json!({
        "task_id": task.id,
        "task_code": format!("IMP-{:06}", task.id),
        "status": task.status,
        "filename": format!("import_task_{}_errors.log", task.id),
        "content": error_log,
    }))))
}
