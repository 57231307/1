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
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::services::import_export_service::{ExportQuery, ImportExportService, MAX_CSV_BYTES};
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
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ImportTaskItem>>>, AppError> {
    // 导入任务功能暂返回空列表，后续可接入数据库任务记录
    Ok(Json(ApiResponse::success(vec![])))
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
