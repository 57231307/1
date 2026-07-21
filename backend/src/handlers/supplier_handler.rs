
use crate::middleware::auth_context::AuthContext;
use crate::services::supplier_service::{
    CreateContactRequest, CreateQualificationRequest, CreateSupplierRequest, SupplierQueryParams,
    SupplierService, UpdateContactRequest, UpdateSupplierRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
// V15 P0-S15/P0-S12 补齐（Batch 474）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{
    build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use validator::Validate;

/// 查询供应商列表
pub async fn list_suppliers(
    Query(params): Query<SupplierQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let result = service.list_suppliers(params, Some(&data_scope_ctx)).await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(result).map_err(AppError::from)?,
    )))
}

/// 获取供应商详情
pub async fn get_supplier(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let supplier = service.get_supplier(id, Some(&data_scope_ctx)).await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(supplier).map_err(AppError::from)?,
    )))
}

/// 创建供应商
#[axum::debug_handler]
pub async fn create_supplier(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSupplierRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()?;

    let service = SupplierService::new(state.db.clone());

    let supplier = service.create_supplier(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(supplier).map_err(AppError::from)?,
        "供应商创建成功",
    )))
}

/// 更新供应商
#[axum::debug_handler]
pub async fn update_supplier(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateSupplierRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_supplier + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_supplier(id, Some(&data_scope_ctx)).await?;

    let supplier = service.update_supplier(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(supplier).map_err(AppError::from)?,
        "供应商更新成功",
    )))
}

/// 删除供应商
pub async fn delete_supplier(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_supplier + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_supplier(id, Some(&data_scope_ctx)).await?;
    service.delete_supplier(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "供应商删除成功",
    )))
}

/// 切换供应商状态
#[axum::debug_handler]
pub async fn toggle_supplier_status(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ToggleStatusRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());

    let supplier = service
        .toggle_supplier_status(id, req.enable, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(supplier).map_err(AppError::from)?,
        if req.enable {
            "供应商已启用"
        } else {
            "供应商已停用"
        },
    )))
}

/// 切换状态请求
#[derive(Debug, Deserialize)]
pub struct ToggleStatusRequest {
    pub enable: bool,
}

// ==================== 供应商联系人管理 Handler ====================

/// 获取供应商联系人列表
pub async fn list_supplier_contacts(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let contacts = service.list_supplier_contacts(supplier_id).await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(contacts).map_err(AppError::from)?,
    )))
}

/// 创建供应商联系人
#[axum::debug_handler]
pub async fn create_supplier_contact(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateContactRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()?;

    let service = SupplierService::new(state.db.clone());

    let contact = service
        .create_supplier_contact(supplier_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact).map_err(AppError::from)?,
        "联系人创建成功",
    )))
}

/// 更新供应商联系人
#[axum::debug_handler]
pub async fn update_supplier_contact(
    Path((_supplier_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateContactRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());

    let contact = service
        .update_supplier_contact(contact_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact).map_err(AppError::from)?,
        "联系人更新成功",
    )))
}

/// 删除供应商联系人
pub async fn delete_supplier_contact(
    Path((_supplier_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    service
        .delete_supplier_contact(contact_id, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "联系人删除成功",
    )))
}

// ==================== 供应商资质管理 Handler ====================

/// 获取供应商资质列表
///
/// 批次 118 P2-9 修复：原 handler 返回硬编码空数组 `serde_json::json!([])`，
/// 违反规则 0（真实实现强制）。改为真实调用 service.list_supplier_qualifications，
/// 从 supplier_qualification 表查询并返回数据。
pub async fn list_supplier_qualifications(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let qualifications = service.list_supplier_qualifications(supplier_id).await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(qualifications).map_err(AppError::from)?,
    )))
}

/// 创建供应商资质
///
/// 批次 118 P2-9 修复：原 handler 返回拼接的假数据 `{"supplier_id": ..., "qualification": req}`，
/// 违反规则 0（真实实现强制）。改为真实调用 service.create_supplier_qualification，
/// 持久化到 supplier_qualification 表并返回真实记录。
#[axum::debug_handler]
pub async fn create_supplier_qualification(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualificationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()?;

    let service = SupplierService::new(state.db.clone());
    let qualification = service
        .create_supplier_qualification(supplier_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(qualification).map_err(AppError::from)?,
        "资质创建成功",
    )))
}

/// V15 P0-S12 + P0-S15 新增（Batch 474）：供应商列表导出为带水印的 xlsx
///
/// 端点：`GET /api/v1/suppliers/export`
///
/// 设计要点：
/// - 复用 `list_suppliers` 的查询参数（SupplierQueryParams）
/// - 通过 `SupplierService::list_suppliers` 一次性查询（page_size=10000 防 OOM）
/// - 行级数据权限：与 `list_suppliers` 一致，调用 `to_data_scope_context`
/// - 水印：操作员（AuthContext.username）+ 导出时间（ISO8601）+ 资源类型说明
///   - IP 暂为 None（middleware 未把 client_ip 注入 AuthContext，后续批次补齐）
///
/// 规则 3：导出统一使用 xlsx 格式（含水印），错误用 AppError 表达。
pub async fn export_suppliers(
    Query(mut params): Query<SupplierQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // V15 P0-S12：复用 list 逻辑，page_size 取上限 10000 防止单次导出过大
    let items = query_suppliers_for_export(&state, &auth, &mut params).await?;
    let row_count = items.len();

    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|s| serde_json::to_value(s).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;
    let table = build_suppliers_table(&items_json);

    let watermark = build_suppliers_watermark(&auth, row_count);
    let filename = format!(
        "suppliers_export_{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );

    record_suppliers_export_audit(&state, &auth, row_count);

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}

/// 查询供应商列表用于导出（强制 page=1, page_size=10000）
async fn query_suppliers_for_export(
    state: &AppState,
    auth: &AuthContext,
    params: &mut SupplierQueryParams,
) -> Result<Vec<crate::models::supplier::Model>, AppError> {
    params.page = Some(1);
    params.page_size = Some(10000);
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    let result = service
        .list_suppliers(params.clone(), Some(&data_scope_ctx))
        .await?;
    Ok(result.items)
}

/// 供应商导出表头（16 列）
fn suppliers_export_headers() -> Vec<String> {
    vec![
        "供应商编码".to_string(),
        "供应商名称".to_string(),
        "简称".to_string(),
        "类型".to_string(),
        "统一社会信用代码".to_string(),
        "法人代表".to_string(),
        "联系电话".to_string(),
        "邮箱".to_string(),
        "注册地址".to_string(),
        "经营地址".to_string(),
        "纳税人类型".to_string(),
        "开户行".to_string(),
        "银行账号".to_string(),
        "等级".to_string(),
        "状态".to_string(),
        "创建时间".to_string(),
    ]
}

/// 从 serde_json::Value 提取供应商行数据
fn build_supplier_row(s: &serde_json::Value) -> Vec<String> {
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
        get_str("supplier_code"),
        get_str("supplier_name"),
        get_str("supplier_short_name"),
        get_str("supplier_type"),
        get_str("credit_code"),
        get_str("legal_representative"),
        get_str("contact_phone"),
        get_str("email"),
        get_str("registered_address"),
        get_str("business_address"),
        get_str("taxpayer_type"),
        get_str("bank_name"),
        get_str("bank_account"),
        get_str("grade"),
        get_str("status"),
        get_str("created_at"),
    ]
}

/// 构造供应商列表 xlsx 表格
fn build_suppliers_table(items_json: &[serde_json::Value]) -> XlsxTable {
    XlsxTable {
        sheet_name: "供应商列表".to_string(),
        headers: suppliers_export_headers(),
        rows: items_json.iter().map(build_supplier_row).collect(),
    }
}

/// 构造供应商导出水印（操作员 + 导出时间 + 资源说明；IP 暂为 None）
fn build_suppliers_watermark(auth: &AuthContext, row_count: usize) -> WatermarkConfig {
    WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None, // 后续批次从 ConnectInfo 提取
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("供应商列表导出（共 {} 条）", row_count)),
    }
}

/// 异步记录供应商导出操作（审计自身，best-effort 不阻塞响应）
fn record_suppliers_export_audit(state: &AppState, auth: &AuthContext, row_count: usize) {
    use crate::models::audit_log::{OperationType, Severity};
    use crate::services::audit_log_service::{AuditEvent, AuditLogService};
    use std::sync::Arc;
    // V15 P0-S12：异步记录导出操作
    let svc = AuditLogService::new(state.db.clone());
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("supplier".to_string()),
        resource_id: None,
        resource_name: Some("供应商列表导出".to_string()),
        description: Some(format!("导出 {} 条供应商数据（含水印）", row_count)),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/suppliers/export".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    Arc::new(svc).record_async(event, None);
}
