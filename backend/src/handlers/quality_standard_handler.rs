
use crate::middleware::auth_context::AuthContext;
use crate::models::quality_standard;
use crate::services::quality_standard_service::QualityStandardService;
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
use chrono::NaiveDate;
use serde::Deserialize;
use tracing::info;

/// 质量标准查询参数 DTO
// V15 P0-S12 修复（Batch 475d）：派生 Clone，export_standards 需要 clone 后覆盖分页参数用于全量导出
#[derive(Debug, Clone, Deserialize)]
pub struct QualityStandardQuery {
    pub standard_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建质量标准请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateQualityStandardRequest {
    /// 标准编码
    pub standard_code: Option<String>,
    /// 标准名称
    pub standard_name: String,
    /// 标准类型：product（产品标准）或 process（工艺标准）
    pub standard_type: Option<String>,
    /// 版本号
    pub version: Option<String>,
    /// 标准内容
    pub content: Option<String>,
    /// 生效日期，格式：YYYY-MM-DD
    pub effective_date: Option<String>,
    /// 失效日期，格式：YYYY-MM-DD（可选）
    pub expiry_date: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

/// 更新质量标准请求 DTO
#[derive(Debug, Deserialize)]
pub struct UpdateQualityStandardRequest {
    /// 标准名称
    pub standard_name: Option<String>,
    /// 标准类型
    pub standard_type: Option<String>,
    /// 标准内容
    pub content: Option<String>,
    /// 状态：draft, approved, published, rejected
    pub status: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

/// 创建版本历史请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateVersionHistoryRequest {
    /// 标准ID
    pub standard_id: i32,
    /// 新版本号
    pub version: String,
    /// 变更原因
    pub change_reason: String,
    /// 变更内容
    pub change_content: String,
}

/// 质量标准审批请求 DTO
#[derive(Debug, Deserialize)]
pub struct QualityApproveRequest {
    /// 审批意见
    pub approval_comment: Option<String>,
}

/// 质量标准驳回请求 DTO（批次 157d-2 新增）
#[derive(Debug, Deserialize)]
pub struct QualityRejectRequest {
    /// 驳回原因
    pub reject_reason: Option<String>,
}

/// 获取质量标准列表
pub async fn list_standards(
    Query(params): Query<QualityStandardQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_standard::Model>>>, AppError> {
    info!("用户 {} 正在查询质量标准列表", auth.username);

    let service = QualityStandardService::new(state.db.clone());
    let query_params = crate::services::quality_standard_service::QualityStandardQueryParams {
        standard_type: params.standard_type,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (standards, _total) = service.get_standards_list(query_params).await?;
    info!("质量标准列表查询成功，共 {} 条记录", standards.len());

    Ok(Json(ApiResponse::success(standards)))
}

/// 获取质量标准详情
pub async fn get_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!("用户 {} 正在查询质量标准详情：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    let standard = service.get_standard_by_id(id).await?;

    info!("质量标准详情查询成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}

/// 创建质量标准
#[axum::debug_handler]
pub async fn create_standard(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualityStandardRequest>,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!(
        "用户 {} 正在创建质量标准：{}",
        auth.username,
        req.standard_code.as_deref().unwrap_or("自动生成")
    );

    let service = QualityStandardService::new(state.db.clone());
    let standard = service
        .create_standard(
            crate::services::quality_standard_service::CreateQualityStandardRequest {
                standard_code: req.standard_code,
                standard_name: req.standard_name,
                standard_type: req.standard_type,
                version: req.version,
                content: req.content,
                effective_date: req
                    .effective_date
                    .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
                expiry_date: req
                    .expiry_date
                    .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("质量标准创建成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}

/// 更新质量标准
#[axum::debug_handler]
pub async fn update_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateQualityStandardRequest>,
) -> Result<Json<ApiResponse<quality_standard::Model>>, AppError> {
    info!("用户 {} 正在更新质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    let standard = service
        .update_standard(
            id,
            crate::services::quality_standard_service::UpdateQualityStandardRequest {
                standard_name: req.standard_name,
                standard_type: req.standard_type,
                content: req.content,
                status: req.status,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("质量标准更新成功：{}", standard.standard_code);
    Ok(Json(ApiResponse::success(standard)))
}

/// 审批质量标准
#[axum::debug_handler]
pub async fn approve_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<QualityApproveRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审批质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service
        .approve_standard(id, auth.user_id, req.approval_comment)
        .await?;

    info!("质量标准审批成功：{}", id);
    Ok(Json(ApiResponse::success("审批成功".to_string())))
}

/// 驳回质量标准（批次 157d-2 新增）
#[axum::debug_handler]
pub async fn reject_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<QualityRejectRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在驳回质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service
        .reject_standard(id, auth.user_id, req.reject_reason)
        .await?;

    info!("质量标准驳回成功：{}", id);
    Ok(Json(ApiResponse::success("驳回成功".to_string())))
}

/// 发布质量标准
#[axum::debug_handler]
pub async fn publish_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在发布质量标准：{}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service.publish_standard(id, auth.user_id).await?;

    info!("质量标准发布成功：{}", id);
    Ok(Json(ApiResponse::success("发布成功".to_string())))
}

/// GET /api/v1/erp/quality-standards/:id/versions - 获取质量标准版本历史
pub async fn list_versions(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    info!("用户 {} 查询质量标准 {} 的版本历史", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    let versions = service.get_version_history(id).await?;

    // 批次 406 修复：序列化失败应传播错误而非返回 Null，避免 API 返回空数据掩盖问题
    let version_list: Vec<serde_json::Value> = versions
        .into_iter()
        .map(|v| serde_json::to_value(v).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ApiResponse::success(version_list)))
}

/// POST /api/v1/erp/quality-standards/versions - 创建版本历史
#[axum::debug_handler]
pub async fn create_version_history(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateVersionHistoryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 为质量标准 {} 创建新版本",
        auth.username, req.standard_id
    );

    let service = QualityStandardService::new(state.db.clone());

    let create_req = crate::services::quality_standard_service::CreateVersionHistoryRequest {
        standard_id: req.standard_id,
        version: req.version,
        change_reason: req.change_reason,
        change_content: req.change_content,
    };

    let version = service
        .create_version_history(create_req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(version)?,
        "版本历史创建成功",
    )))
}

/// DELETE /api/v1/erp/quality-standards/:id - 删除质量标准
#[axum::debug_handler]
pub async fn delete_standard(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除质量标准 {}", auth.username, id);

    let service = QualityStandardService::new(state.db.clone());
    service.delete_standard(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "质量标准已删除",
    )))
}

/// 质量标准导出表头（13 列）
fn standards_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "标准编码".to_string(),
        "标准名称".to_string(),
        "标准类型".to_string(),
        "版本".to_string(),
        "生效日期".to_string(),
        "到期日期".to_string(),
        "状态".to_string(),
        "审批人ID".to_string(),
        "审批时间".to_string(),
        "创建人ID".to_string(),
        "创建时间".to_string(),
        "更新时间".to_string(),
    ]
}

/// 从 serde_json::Value 提取质量标准行数据
fn build_standard_row(s: &serde_json::Value) -> Vec<String> {
    let get_str = |key: &str| -> String {
        s.get(key)
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
        get_str("standard_code"),
        get_str("standard_name"),
        get_str("standard_type"),
        get_str("version"),
        get_str("effective_date"),
        get_str("expiry_date"),
        get_str("status"),
        get_str("approved_by"),
        get_str("approved_at"),
        get_str("created_by"),
        get_str("created_at"),
        get_str("updated_at"),
    ]
}

/// 构造质量标准列表 xlsx 表格
fn build_standards_table(standards_json: &[serde_json::Value]) -> XlsxTable {
    XlsxTable {
        sheet_name: "质量标准列表".to_string(),
        headers: standards_export_headers(),
        rows: standards_json.iter().map(build_standard_row).collect(),
    }
}

/// 异步记录质量标准导出操作（审计自身，best-effort 不阻塞响应）
fn record_standards_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    filename: &str,
) {
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("quality_standard".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出质量标准列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/quality-standards/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    svc.record_async(event, None);
}

/// GET /api/v1/erp/quality-standards/export - 导出质量标准列表（带水印 + 异步审计日志）
///
/// V15 P0-S12 修复（Batch 475d）：导出接入后端
/// - 注入水印（operator/exported_at/extra 含条数）
/// - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_standards_list 取全量数据（page=1/page_size=10000）
/// - 不复用 list_standards handler 逻辑（保持单一职责）
pub async fn export_standards(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<QualityStandardQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = QualityStandardService::new(state.db.clone());
    // V15 P0-S12 修复（Batch 475d）：导出全量数据
    let query_params = crate::services::quality_standard_service::QualityStandardQueryParams {
        standard_type: query.standard_type,
        status: query.status,
        page: 1,
        page_size: 10000,
    };
    let (standards, _total) = service.get_standards_list(query_params).await?;
    let row_count = standards.len();
    // 序列化为 JSON 以统一字段处理
    let standards_json: Vec<serde_json::Value> = standards
        .into_iter()
        .map(|s| serde_json::to_value(s).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;
    let table = build_standards_table(&standards_json);
    let filename = format!(
        "quality_standards_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_standards_export_audit(&state, &auth, row_count, &filename);
    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("质量标准导出（共 {} 条）", row_count)),
    };
    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
