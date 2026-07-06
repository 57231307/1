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
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::import_export_service::{
    ExportQuery, ImportExportService, ImportResult, MAX_CSV_BYTES,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// CSV 导入请求
///
/// 安全约束：data 字段使用 validator crate 做长度上限校验（10MB），
/// 防止已认证用户发送超大请求触发 OOM DoS。
#[derive(Debug, Deserialize, Validate)]
pub struct CsvImportRequest {
    pub import_type: String,
    // validator 0.16 的 length(max = ...) 不支持 Rust 表达式，只能用整数字面量。
    // 10 * 1024 * 1024 = 10485760 字节 = 10 MB。
    #[validate(length(max = 10485760, message = "CSV 数据超过 10MB 上限"))]
    pub data: String, // CSV 格式的字符串
}

/// Excel 导入请求
///
/// 安全约束：data 行数使用 validator crate 做上限校验（1 万行），
/// 单元格/列数限制由 handler 入口早期校验 + service 层 defense-in-depth 双重把关。
#[derive(Debug, Deserialize, Validate)]
pub struct ExcelImportRequest {
    pub import_type: String,
    #[validate(length(max = 10_000, message = "Excel 数据超过 1 万行上限"))]
    pub data: Vec<Vec<String>>, // 二维数组
}

/// POST /api/v1/erp/import/csv - CSV导入
pub async fn import_csv(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CsvImportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 安全漏洞 #8 修复：DTO 校验失败（数据超过 10MB）→ 友好错误
    // validator crate 自动从 #[validate(length(max = ...))] 注解生成校验逻辑，
    // 错误通过 `?` 操作符转为 AppError::ValidationError 返回。
    req.validate()?;

    // 安全漏洞 #8 修复：handler 入口早期校验（defense-in-depth 第二层）
    // 即使 DTO 校验被绕过（例如：手写请求绕过 axum 提取器层），
    // 本入口仍以毫秒级速度拒绝超大数据，避免后续解析逻辑耗尽内存。
    if req.data.len() > MAX_CSV_BYTES {
        return Err(AppError::validation(format!(
            "CSV 数据超过 {} 字节上限：当前 {} 字节",
            MAX_CSV_BYTES,
            req.data.len()
        )));
    }

    let service = ImportExportService::new(state.db.clone());

    // 获取导入模板
    let template = ImportExportService::get_import_template(&req.import_type)?;

    // 解析CSV数据
    let rows = ImportExportService::parse_csv(&req.data)?;

    // 批次 127 v8 复审 P2 修复：导入前创建任务记录（status=running）
    // 即使后续验证失败或导入异常，任务表也会落库一条记录，便于追溯历史导入行为。
    let task_id = service
        .create_import_task(&req.import_type, rows.len() as u64, auth.user_id)
        .await?;

    // 验证数据
    let errors = ImportExportService::validate_import_data(&rows, &template);

    if !errors.is_empty() {
        // 验证失败：更新任务记录为 failed 状态（imported=0, failed=rows.len()）
        let fail_result = ImportResult {
            imported: 0,
            failed: rows.len() as u64,
            errors,
        };
        // 任务更新失败不阻断主流程（仅 tracing::warn!），保证用户得到原始验证错误响应
        if let Err(e) = service.update_import_task(task_id, &fail_result).await {
            tracing::warn!(error = %e, task_id, "更新导入任务记录为 failed 状态失败");
        }
        return Ok(Json(ApiResponse::success(serde_json::to_value(fail_result)?)));
    }

    // 执行实际导入
    let result = service
        .import_data(&req.import_type, &rows, auth.user_id)
        .await?;

    // 导入完成：更新任务记录（status=success/failed/partial）
    if let Err(e) = service.update_import_task(task_id, &result).await {
        tracing::warn!(error = %e, task_id, "更新导入任务记录为完成状态失败");
    }

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
    // 安全漏洞 #8 修复：DTO 校验失败（行数超过 1 万行）→ 友好错误
    req.validate()?;

    // 安全漏洞 #8 修复：handler 入口早期校验
    // - 行数：DTO 校验已覆盖；本处冗余判断以防 validator crate 未来 API 变化
    // - 列数 / 单元格长度：validator crate 的 `length` 不直接支持嵌套 Vec，
    //   在 handler 入口做精确校验并返回友好中文错误
    use crate::services::import_export_service::{MAX_CELL_LEN, MAX_EXCEL_COLS, MAX_EXCEL_ROWS};

    if req.data.len() > MAX_EXCEL_ROWS {
        return Err(AppError::validation(format!(
            "Excel 数据超过 {} 行上限：当前 {} 行",
            MAX_EXCEL_ROWS,
            req.data.len()
        )));
    }
    for (row_idx, row) in req.data.iter().enumerate() {
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

    let service = ImportExportService::new(state.db.clone());

    // 获取导入模板
    let template = ImportExportService::get_import_template(&req.import_type)?;

    // 批次 127 v8 复审 P2 修复：导入前创建任务记录（status=running）
    let task_id = service
        .create_import_task(&req.import_type, req.data.len() as u64, auth.user_id)
        .await?;

    // 验证数据
    let errors = ImportExportService::validate_import_data(&req.data, &template);

    if !errors.is_empty() {
        // 验证失败：更新任务记录为 failed 状态
        let fail_result = ImportResult {
            imported: 0,
            failed: req.data.len() as u64,
            errors,
        };
        if let Err(e) = service.update_import_task(task_id, &fail_result).await {
            tracing::warn!(error = %e, task_id, "更新导入任务记录为 failed 状态失败");
        }
        return Ok(Json(ApiResponse::success(serde_json::to_value(fail_result)?)));
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

/// GET /api/v1/erp/export/csv/:export_type - 数据导出（xlsx）
pub async fn export_csv(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ImportExportService::new(state.db.clone());

    let (headers, data) = service.export_data(&export_type, &query).await?;

    // 规则 3：导出统一使用 xlsx 格式
    let xlsx_bytes = ImportExportService::generate_xlsx(&headers, &data)?;
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD.encode(&xlsx_bytes);

    // P1 8-6 修复：export_csv 补审计日志（原仅 tracing::info）
    // 修复背景：原 export_csv 仅 tracing::info 输出，未调 audit_log_service，
    // 数据导出无审计落库，无法追溯谁导出了什么数据。
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
        request_path: Some(format!("/api/v1/erp/export/csv/{}", export_type)),
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

/// 导入模板列表项
#[derive(Debug, serde::Serialize)]
pub struct ImportTemplateListItem {
    pub import_type: String,
    pub name: String,
    pub description: String,
}

/// 导入任务列表项
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

#[cfg(test)]
mod tests {
    //! 安全漏洞 #8 修复配套单测
    //!
    //! 测试目标：DTO #[validate] 注解在反序列化后能正确拒绝超限数据。
    //! 备注：handler 早期校验的测试需要 mock State/AppState/AuthContext，
    //! 仅测试 DTO 层（不涉及 handler 调用），覆盖率已足够。
    use super::*;
    use crate::services::import_export_service::{MAX_CSV_BYTES, MAX_EXCEL_ROWS};

    /// 漏洞 #8 修复：CSV data 字段超过 10MB → validate() 失败
    #[test]
    fn test_csv_import_request_rejects_exceeding_10mb() {
        // 构造一个 data 字段超过 10MB 的请求
        let big_csv = "a".repeat(MAX_CSV_BYTES + 1);
        let req = CsvImportRequest {
            import_type: "products".to_string(),
            data: big_csv,
        };

        // 期望 validate() 失败（被 #[validate(length(max = 10485760))] 拦截）
        let result = req.validate();
        assert!(
            result.is_err(),
            "漏洞 #8 单测：{} 字节的 CSV data 应被 validate() 拒绝",
            MAX_CSV_BYTES + 1
        );
    }

    /// 漏洞 #8 修复：Excel data 行数超过 1 万行 → validate() 失败
    #[test]
    fn test_excel_import_request_rejects_exceeding_10k_rows() {
        // 构造一个 data 字段超过 1 万行的请求
        let mut rows = Vec::with_capacity(MAX_EXCEL_ROWS + 1);
        for _ in 0..=MAX_EXCEL_ROWS {
            rows.push(vec!["P001".to_string(), "name".to_string()]);
        }
        let req = ExcelImportRequest {
            import_type: "products".to_string(),
            data: rows,
        };

        // 期望 validate() 失败（被 #[validate(length(max = 10_000))] 拦截）
        let result = req.validate();
        assert!(
            result.is_err(),
            "漏洞 #8 单测：{} 行的 Excel data 应被 validate() 拒绝",
            MAX_EXCEL_ROWS + 1
        );
    }

    /// 漏洞 #8 修复：边界值测试 - 10MB 的 CSV 应通过 validate()
    #[test]
    fn test_csv_import_request_accepts_exactly_10mb() {
        // 构造一个 data 字段正好 10MB 的请求
        let csv = "a".repeat(MAX_CSV_BYTES);
        let req = CsvImportRequest {
            import_type: "products".to_string(),
            data: csv,
        };

        // 期望 validate() 成功
        let result = req.validate();
        assert!(
            result.is_ok(),
            "漏洞 #8 单测：恰好 {} 字节的 CSV data 应通过 validate()",
            MAX_CSV_BYTES
        );
    }
}
