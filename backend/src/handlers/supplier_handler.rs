use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::{supplier_contact, supplier_qualification};
use crate::services::supplier_service::{
    CreateContactRequest, CreateQualificationRequest, CreateSupplierRequest, SupplierQueryParams,
    SupplierService, UpdateContactRequest, UpdateSupplierRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
// V15 P0-S15/P0-S12 补齐（Batch 474）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
use axum::{
    Json,
    body::Body,
    extract::{DefaultBodyLimit, FromRequest, Multipart, Path, Query, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
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
    let result = service
        .list_suppliers(params, Some(&data_scope_ctx))
        .await?;

    let mut value = serde_json::to_value(result).map_err(AppError::from)?;
    // P1-08-5：非管理员对供应商列表手机号/邮箱脱敏（含 contacts 子数组）
    value =
        crate::utils::field_mask::mask_contact_fields_batch_for_role(value, auth.role_id, "list");
    value =
        crate::utils::field_mask::mask_contact_fields_batch_for_role(value, auth.role_id, "items");
    value =
        crate::utils::field_mask::mask_contact_fields_batch_for_role(value, auth.role_id, "data");
    // 顶层若为数组也脱敏
    if let Some(arr) = value.as_array_mut() {
        for item in arr.iter_mut() {
            *item =
                crate::utils::field_mask::mask_contact_fields_for_role(item.clone(), auth.role_id);
        }
    }

    // B12-P2-2：字段级权限过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "supplier")
            .await
        {
            // 如果是数组，批量过滤
            if let Some(arr) = value.as_array_mut() {
                state.data_permission_service.filter_fields_batch(
                    arr,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            } else {
                state.data_permission_service.filter_fields(
                    &mut value,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
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

    let mut value = serde_json::to_value(supplier).map_err(AppError::from)?;
    // P1-08-5：非管理员对供应商详情手机号/邮箱脱敏
    value = crate::utils::field_mask::mask_contact_fields_for_role(value, auth.role_id);

    // B12-P2-2：字段级权限过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "supplier")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut value,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        }
    }

    Ok(Json(ApiResponse::success(value)))
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
    // 与 update/delete 同源 IDOR 防护：状态改写前校验资源归属（越权 403）。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_supplier(id, Some(&data_scope_ctx)).await?;

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
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ToggleStatusRequest {
    pub enable: bool,
}

// ==================== 供应商联系人管理 Handler ====================

/// 获取供应商联系人列表
/// IDOR 防护：子资源经父资源做归属门控（与同文件资质四端点、inventory_adjustment_handler
/// 同源范式）——先按 data_scope 校验路径供应商归属（与同域 list_suppliers 行级权限同源），
/// 越权 403，避免任意 supplier_id 枚举他人联系人手机号/邮箱。
pub async fn list_supplier_contacts(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<supplier_contact::Model>>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    let contacts = service.list_supplier_contacts(supplier_id).await?;

    Ok(Json(ApiResponse::success(contacts)))
}

/// 创建供应商联系人
/// IDOR 防护：写入前经父供应商做归属门控（与 list/update/delete 同源），越权 403。
#[axum::debug_handler]
pub async fn create_supplier_contact(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateContactRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()?;

    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;

    let contact = service
        .create_supplier_contact(supplier_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact).map_err(AppError::from)?,
        "联系人创建成功",
    )))
}

/// 更新供应商联系人
/// IDOR 防护：路径 (supplier_id, contact_id) 双键——先经父供应商归属门控（data_scope），
/// 再由 service 校验联系人确属该供应商，不匹配返回用户可见业务错误，
/// 防止以 contact_id 单键跨供应商改写他人联系人。
#[axum::debug_handler]
pub async fn update_supplier_contact(
    Path((supplier_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateContactRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;

    let contact = service
        .update_supplier_contact(supplier_id, contact_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact).map_err(AppError::from)?,
        "联系人更新成功",
    )))
}

/// 删除供应商联系人
/// IDOR 防护：与 update 同源，先父门控再归属校验，防止单键枚举删除他人联系人。
pub async fn delete_supplier_contact(
    Path((supplier_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    service
        .delete_supplier_contact(supplier_id, contact_id, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "联系人删除成功",
    )))
}

// ==================== 供应商资质管理 Handler ====================

/// 获取供应商资质列表；批次 118 P2-9 修复：原 handler 返回硬编码空数组 `serde_json::json!([])`， 违反规则 0（真实实现强制）
/// 改为真实调用 service.list_supplier_qualifications， 从 supplier_qualification 表查询并返回数据。
/// IDOR 防护：子资源经父资源做归属门控（同 inventory_adjustment_handler 范式）——
/// 先按 data_scope 校验路径供应商归属（与同域 list_suppliers 行级权限同源），越权 403，
/// 避免任意 supplier_id 枚举他人资质。
pub async fn list_supplier_qualifications(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<supplier_qualification::Model>>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    let qualifications = service.list_supplier_qualifications(supplier_id).await?;

    Ok(Json(ApiResponse::success(qualifications)))
}

/// 创建供应商资质；批次 118 P2-9 修复：原 handler 返回拼接的假数据 `{"supplier_id": ..., "qualification": req}`， 违反规则
/// 0（真实实现强制）。改为真实调用 service.create_supplier_qualification， 持久化到 supplier_qualification 表并返回真实记录。
/// IDOR 防护：写入前经父供应商做归属门控（与 update/delete/list 同源），越权 403。
#[axum::debug_handler]
pub async fn create_supplier_qualification(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualificationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()?;

    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    let qualification = service
        .create_supplier_qualification(supplier_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(qualification).map_err(AppError::from)?,
        "资质创建成功",
    )))
}

/// 更新供应商资质
/// IDOR 防护：路径 (supplier_id, qualification_id) 双键——先经父供应商归属门控（data_scope，
/// 同 inventory_adjustment_handler 范式），再由 service 校验资质确属该供应商，
/// 不匹配返回用户可见业务错误，防止以 qualification_id 单键枚举改写他人资质。
#[axum::debug_handler]
pub async fn update_supplier_qualification(
    Path((supplier_id, qualification_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualificationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()?;
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    let qualification = service
        .update_supplier_qualification(supplier_id, qualification_id, req)
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(qualification).map_err(AppError::from)?,
        "资质更新成功",
    )))
}

/// 删除供应商资质
/// IDOR 防护：与 update 同源，先父门控再归属校验，防止单键枚举删除他人资质。
#[axum::debug_handler]
pub async fn delete_supplier_qualification(
    Path((supplier_id, qualification_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    service
        .delete_supplier_qualification(supplier_id, qualification_id)
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({ "deleted_id": qualification_id }),
        "资质删除成功",
    )))
}

/// V15 P0-S12 + P0-S15 新增（Batch 474）：供应商列表导出为带水印的 xlsx；端点：`GET /api/v1/suppliers/export`；设计要点： - 复用 `list_suppliers` 的查询参数（SupplierQueryParams） - 通过 `SupplierService::list_suppliers` 一次性查询（page_size=10000 防 OOM） -
/// 行级数据权限：与 `list_suppliers` 一致，调用 `to_data_scope_context` - 水印：操作员（AuthContext.username）+ 导出时间（ISO8601）+ 资源类型说明 - IP 暂为 None（middleware 未把 client_ip 注入 AuthContext，后续批次补齐）；规则 3：导出统一使用 xlsx 格式（含水印），错误用 AppError 表达。
pub async fn export_suppliers(
    Query(mut params): Query<SupplierQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：校验审批令牌（在 params 被 move 前提取）
    let download_token = params.download_token.clone();
    let approval =
        crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
            .enforce_export_download(download_token.as_deref(), "supplier")
            .await?;

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

    // 敏感导出 fail-closed：记录令牌消费
    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(
            approval.id,
            filename.clone(),
            row_count as i64,
            String::new(),
        )
        .await;

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

/// batch-13 P2：查询供应商账户余额
/// GET /api/v1/erp/suppliers/:id/balance
pub async fn get_supplier_balance(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(
        user_id = auth.user_id,
        supplier_id = id,
        "查询供应商账户余额"
    );
    let service = SupplierService::new(state.db.clone());
    let balance = service.get_supplier_balance(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(balance)?)))
}

/// batch-13 P2：检测异常大额订单
/// GET /api/v1/erp/suppliers/abnormal-orders
pub async fn detect_abnormal_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<AbnormalOrderQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "检测异常大额订单");
    let service = SupplierService::new(state.db.clone());
    let threshold = params.threshold_ratio.unwrap_or(3.0);
    let threshold_decimal =
        rust_decimal::Decimal::try_from(threshold).unwrap_or(rust_decimal::Decimal::from(3));
    let abnormal_orders = service.detect_abnormal_orders(threshold_decimal).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "abnormal_orders": abnormal_orders,
        "total": abnormal_orders.len(),
        "threshold_ratio": threshold,
    }))))
}

/// 异常订单查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct AbnormalOrderQuery {
    pub threshold_ratio: Option<f64>,
}

/// batch-13 P3: 供货历史查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct PurchaseHistoryQuery {
    pub limit: Option<u64>,
}

/// batch-13 P3: 供货历史查询端点
/// GET /api/v1/erp/suppliers/:id/purchase-history
pub async fn get_supplier_purchase_history(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Query(params): Query<PurchaseHistoryQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, supplier_id = id, "查询供货历史");
    let service = SupplierService::new(state.db.clone());
    let history = service
        .get_supplier_purchase_history(id, params.limit)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "history": history,
        "total": history.len(),
    }))))
}

// ==================== 供应商资质附件（真上传 + 鉴权读取） ====================

/// 资质附件扩展名白名单：仅证照扫描件（pdf/jpg/jpeg/png），不放行可执行文件/压缩包/宏文档
const QUALIFICATION_ATTACHMENT_ALLOWED_EXTS: &[&str] = &["pdf", "jpg", "jpeg", "png"];

/// 资质附件大小上限（5MB）：双端校验（前端上传前 + 后端，与头像上传同一"双保险"口径）；
/// 必须小于全局请求体上限（constants.rs MAX_HTTP_BODY_BYTES = 12MB，全局 DefaultBodyLimit
/// 层与本 handler 单点覆写同源引用），
/// 为 multipart 边界开销留足余量，避免超限请求先被全局 body limit 掐断成不可解释的失败。
const MAX_QUALIFICATION_ATTACHMENT_SIZE: usize = 5 * 1024 * 1024;

/// 资质附件落盘目录：独立于头像 uploads/avatars。
/// 安全要点：该目录 **不挂任何匿名静态路由**（对比 /uploads/avatars/{*path}），
/// 文件字节只能经下方鉴权端点 GET 读出（fail-closed：auth 中间件 401 / permission 中间件 403 /
/// handler 内父门控 + 双键归属校验任一不过都拿不到文件）。
const QUALIFICATION_UPLOAD_DIR: &str = "uploads/qualifications";

/// attachment_path 落库 URL 前缀（受控形态 /uploads/qualifications/{supplier_id}_{qualification_id}.{ext}；
/// 该 URL 不对应公开路由，仅作文件名定位，读取仍必须走鉴权端点）
const QUALIFICATION_ATTACHMENT_URL_PREFIX: &str = "/uploads/qualifications/";

/// 依据文件头（magic bytes）识别真实证照文件类型——不信 Content-Type、不信客户端后缀。
/// 无法识别返回 None（伪造/未知内容一律拒绝）。
fn detect_qualification_attachment_magic(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"%PDF") {
        Some("pdf")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else {
        None
    }
}

/// 声明扩展名与 magic 识别结果是否同族（jpg/jpeg 同指 JPEG）。
/// 后缀与内容不同族（如 .png 实为 PDF）视为伪造，拒绝。
fn qualification_ext_matches_magic(ext: &str, magic_kind: &str) -> bool {
    match magic_kind {
        "pdf" => ext == "pdf",
        "jpg" => matches!(ext, "jpg" | "jpeg"),
        "png" => ext == "png",
        _ => false,
    }
}

/// 资质附件病毒扫描检查点：CLAMAV_ENABLED 开关与 ClamAV REST 调用口径与
/// CRM 导入（crm_handler.rs::scan_leads_for_viruses）一致；未启用时如实记录
/// warn 日志（本仓禁止静默），边界：此时文件未经病毒扫描即落盘，生产环境必须
/// 设置 CLAMAV_ENABLED=true 并配置 CLAMAV_URL。
async fn scan_qualification_attachment_for_viruses(data: &[u8]) -> Result<(), AppError> {
    let enabled = std::env::var("CLAMAV_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    if !enabled {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_SKIPPED",
            "CLAMAV_ENABLED 未启用，供应商资质附件上传跳过病毒扫描；生产环境应设置 CLAMAV_ENABLED=true 并配置 CLAMAV_URL"
        );
        return Ok(());
    }

    // 决策定案 #4：扫描依赖故障族（未配置/不可达/非 2xx/响应读取失败）不是
    // 「我方服务器坏了」（500 InternalError），而是外部扫描依赖不可用——统一走
    // AppError::service_unavailable（HTTP 503 / code=SERVICE_UNAVAILABLE / 公网脱敏文案）。
    // 真实原因只进 tracing::warn（CLAMAV_SCAN_UNAVAILABLE 事件标签），不外泄 URL/端口/配置键名；
    // fail-closed 不动摇：以下每一条故障路径都在落盘（步骤 8）之前 return Err。
    let clamav_url = std::env::var("CLAMAV_URL").unwrap_or_default();
    if clamav_url.trim().is_empty() {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_UNAVAILABLE",
            "CLAMAV_ENABLED 已启用但扫描服务地址未配置或为空，资质附件上传已拒绝（fail-closed，未落盘）"
        );
        return Err(AppError::service_unavailable(
            "病毒扫描依赖不可用：扫描服务地址未配置（资质附件上传），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
        ));
    }

    let client = reqwest::Client::new();
    let scan_url = format!("{}/scan", clamav_url.trim_end_matches('/'));
    let traceparent = crate::observability::trace_context::traceparent_from_current_span();
    let response = client
        .post(&scan_url)
        .header("Content-Type", "application/octet-stream")
        .header("traceparent", traceparent)
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(
                target: "security_audit",
                event = "CLAMAV_SCAN_UNAVAILABLE",
                error = %e,
                "病毒扫描服务不可达（连接失败/超时），资质附件上传已拒绝（fail-closed，未落盘）"
            );
            AppError::service_unavailable(
                "病毒扫描依赖不可用：扫描服务不可达（资质附件上传），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
            )
        })?;

    if !response.status().is_success() {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_UNAVAILABLE",
            upstream_status = %response.status(),
            "病毒扫描服务返回非 2xx，资质附件上传已拒绝（fail-closed，未落盘）"
        );
        return Err(AppError::service_unavailable(
            "病毒扫描依赖不可用：扫描服务返回非 2xx（资质附件上传），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
        ));
    }

    let body = response.text().await.map_err(|e| {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_UNAVAILABLE",
            error = %e,
            "读取病毒扫描响应失败，资质附件上传已拒绝（fail-closed，未落盘）"
        );
        AppError::service_unavailable(
            "病毒扫描依赖不可用：扫描响应读取失败（资质附件上传），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
        )
    })?;

    // ClamAV REST 返回 "stream: OK" 表示无病毒
    if body.contains("OK") {
        tracing::info!(
            target: "security_audit",
            event = "CLAMAV_SCAN_PASSED",
            "供应商资质附件通过 ClamAV 病毒扫描"
        );
        Ok(())
    } else {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_REJECTED",
            "供应商资质附件未通过 ClamAV 病毒扫描，已拒绝落盘"
        );
        Err(AppError::business_displayable(
            "资质附件未通过病毒扫描，已被拒绝，请向供应商索取干净的文件后重新上传",
        ))
    }
}

/// POST /api/v1/erp/purchase/suppliers/{supplier_id}/qualifications/{qualification_id}/attachment
/// 资质附件真上传（multipart，文件字段名 file）。安全校验清单（逐条对齐先例）：
/// 1. 父资源归属门控 + data_scope：`get_supplier(supplier_id, Some(&ctx))`，与本域四个
///    资质端点同源写法（越权由 get_supplier 出 403/404）；
/// 2. (supplier_id, qualification_id) 双键归属校验：service 层 `get_supplier_qualification`，
///    错配返回 `business_displayable` 用户可见文案（`business` 出参会被脱敏）；先于任何
///    落盘 IO 执行，越权请求不产生垃圾文件；
/// 3. 大小上限：双端校验（前端同值 5MB），超限显式拒绝；
/// 4. 扩展名白名单：pdf/jpg/jpeg/png（证照场景，不放行可执行/压缩包）；
/// 5. magic bytes 校验：真实文件头必须命中 PDF/JPEG/PNG 之一，且与声明扩展名同族——
///    伪造 Content-Type 或后缀但内容不符一律拒绝（对齐 crm_handler.rs 导入范式）；
/// 6. ClamAV 病毒扫描：开关口径同 CRM 导入，未启用如实 warn（不静默）；
/// 7. 文件名服务端生成 `{supplier_id}_{qualification_id}.{ext}`：禁用客户端原名
///    （防路径遍历与跨记录覆盖，对齐头像上传范式）；
/// 8. 落库前 VARCHAR(500) 列宽显式校验（service 层），杜绝越界裸 500。
#[axum::debug_handler]
pub async fn upload_supplier_qualification_attachment(
    Path((supplier_id, qualification_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    request: Request,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    // 通道前提：axum Multipart 提取器自带 2MB 默认请求体上限，若不覆写，超限文件在
    // `Field::bytes()` 处即被 LengthLimitError 掐断——下方第 4) 步的 5MB 显式大小校验
    // 根本执行不到，"体积超限"这一输入校验族的拒绝会畸形成流读取失败的 BUSINESS_ERROR
    // （族别错判）。在此单点把上限覆写为全局 HTTP 请求体上限（复用权威常量
    // crate::constants::MAX_HTTP_BODY_BYTES，与全局中间件层同源，不造第二套上限值）：
    // 该范围内字节流可完整读入，"超过 5MB"的业务判定唯一落点即第 4) 步显式校验。
    let mut request = request;
    DefaultBodyLimit::max(crate::constants::MAX_HTTP_BODY_BYTES).apply(&mut request);
    let mut multipart = <Multipart as FromRequest<_>>::from_request(request, &())
        .await
        .map_err(|e| {
            tracing::warn!(
                user_id = auth.user_id,
                supplier_id,
                qualification_id,
                error = %e,
                "资质附件上传请求不是合法 multipart（Content-Type 缺失或无 boundary）"
            );
            // 请求形态非法属输入格式校验族（VALIDATION），文案只述请求形态事实
            AppError::validation_displayable(
                "上传请求必须是合法的 multipart/form-data（需包含 boundary）",
            )
        })?;
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    // 1) 父资源归属门控 + 行级数据权限（与 list/create/update/delete 资质端点同源）
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    // 2) 双键归属校验（先于任何落盘 IO）
    service
        .get_supplier_qualification(supplier_id, qualification_id)
        .await?;

    // 3) 读取 multipart 的 file 字段（字段名非 file 的忽略；未提供显式报错，不静默跳过）
    let mut received: Option<(String, Vec<u8>)> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            error = %e,
            "资质附件 multipart 读取失败"
        );
        AppError::business_displayable("附件文件读取失败，请重新上传")
    })? {
        if field.name() != Some("file") {
            continue;
        }
        let client_file_name = field.file_name().unwrap_or("").to_string();
        let data = field.bytes().await.map_err(|e| {
            tracing::warn!(
                user_id = auth.user_id,
                supplier_id,
                qualification_id,
                error = %e,
                "资质附件字节流读取失败"
            );
            AppError::business_displayable("附件文件读取失败，请重新上传")
        })?;
        received = Some((client_file_name, data.to_vec()));
        break;
    }
    let (client_file_name, data) = received.ok_or_else(|| {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            "资质附件上传请求缺少 file 字段"
        );
        // 反向族校正：请求体缺少必填 file 字段属「必填缺失」输入校验，归校验族（文案只述请求字段可外显）
        AppError::validation_displayable("请求中未找到附件文件字段（字段名须为 file）")
    })?;

    // 4) 大小上限（与前端上传前校验同值，双保险）
    if data.len() > MAX_QUALIFICATION_ATTACHMENT_SIZE {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            size_bytes = data.len(),
            "资质附件超过大小上限，已拒绝"
        );
        // 反向族校正：上传文件大小超限属「范围/大小」输入校验，归校验族；上限是公开规则可外显
        return Err(AppError::validation_displayable(format!(
            "资质附件不能超过 {}MB",
            MAX_QUALIFICATION_ATTACHMENT_SIZE / 1024 / 1024
        )));
    }

    // 5) 扩展名白名单（仅取后缀本身做门控；真实类型以 magic bytes 为准）
    let ext = std::path::Path::new(&client_file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if !QUALIFICATION_ATTACHMENT_ALLOWED_EXTS.contains(&ext.as_str()) {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            declared_ext = %ext,
            "资质附件扩展名不在白名单，已拒绝"
        );
        // 反向族校正：扩展名不在白名单属「格式」输入校验，归校验族；白名单为公开规则可外显
        return Err(AppError::validation_displayable(
            "资质附件仅支持 pdf/jpg/jpeg/png 格式",
        ));
    }

    // 6) magic bytes 校验：不能只信 Content-Type 或后缀
    let Some(magic_kind) = detect_qualification_attachment_magic(&data) else {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            declared_ext = %ext,
            "资质附件内容 magic bytes 校验失败（非 PDF/JPEG/PNG），已拒绝"
        );
        // 反向族校正：文件内容格式非法属「格式」输入校验，归校验族；
        // 文案已按安全边界去掉内部机制术语（magic bytes），只述用户可理解的格式事实。
        return Err(AppError::validation_displayable(
            "附件内容不是有效的证照文件，仅支持 PDF/JPEG/PNG",
        ));
    };
    if !qualification_ext_matches_magic(&ext, magic_kind) {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            declared_ext = %ext,
            detected_kind = magic_kind,
            "资质附件声明扩展名与实际内容不符（疑似伪造），已拒绝"
        );
        // 反向族校正：声明扩展名与实际内容不符属「格式」输入校验，归校验族；回显的是用户自己的文件名可外显
        return Err(AppError::validation_displayable(format!(
            "附件扩展名 .{} 与文件实际内容不符，请检查后重新上传",
            ext
        )));
    }

    // 7) 病毒扫描（ClamAV，开关口径同 CRM 导入）
    scan_qualification_attachment_for_viruses(&data).await?;

    // 8) 服务端生成文件名并落盘（独立目录；客户端原名不参与任何路径拼接）
    let dir = std::path::Path::new(QUALIFICATION_UPLOAD_DIR);
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| AppError::internal(format!("创建资质附件目录失败：{}", e)))?;
    // 受控文件名单点号形态 `{supplier_id}_{qualification_id}.{ext}`（与上方文档清单第 7 条、
    // 读取端 `{}_{}.` 前缀校验同一权威口径）：`{sid}.{qid}.{ext}` 双点号会使
    // `Path::extension()` 定位、后缀白名单与读取端前缀校验全部漂移，禁止回潮。
    let generated_name = format!("{}_{}.{}", supplier_id, qualification_id, ext);
    let attachment_url = format!("{}{}", QUALIFICATION_ATTACHMENT_URL_PREFIX, generated_name);
    // VARCHAR(500) 列宽防线：在落盘前先显式拒绝超长生成形态，避免磁盘与库不一致
    if attachment_url.len() > SupplierService::MAX_ATTACHMENT_PATH_LEN {
        tracing::error!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            url_len = attachment_url.len(),
            "资质附件受控 URL 超过列宽上限，属部署配置异常"
        );
        return Err(AppError::business_displayable(format!(
            "生成的附件访问地址超过 {} 字符上限，请联系管理员检查部署配置",
            SupplierService::MAX_ATTACHMENT_PATH_LEN
        )));
    }

    // 替换语义：不同扩展名重传时清理该资质旧附件文件，避免目录残留孤儿证照；
    // 旧文件不存在是正常状态（首次上传），清理真实失败如实 warn（不静默、不阻断新文件）
    for stale_ext in QUALIFICATION_ATTACHMENT_ALLOWED_EXTS {
        if *stale_ext == ext {
            continue;
        }
        let stale = dir.join(format!(
            "{}_{}.{}",
            supplier_id, qualification_id, stale_ext
        ));
        match tokio::fs::remove_file(&stale).await {
            Ok(()) => {
                tracing::info!(
                    supplier_id,
                    qualification_id,
                    stale_ext,
                    "资质附件换格式重传，旧附件文件已清理"
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(
                    supplier_id,
                    qualification_id,
                    stale_ext,
                    "资质附件旧文件不存在（首次上传），无需清理"
                );
            }
            Err(e) => {
                tracing::warn!(
                    supplier_id,
                    qualification_id,
                    stale_ext,
                    error = %e,
                    "资质附件旧文件清理失败（新附件不受影响，需运维关注）"
                );
            }
        }
    }

    let file_path = dir.join(&generated_name);
    tokio::fs::write(&file_path, &data)
        .await
        .map_err(|e| AppError::internal(format!("资质附件写入失败：{}", e)))?;

    // 9) 落库回写受控 URL（仅 attachment_path 单列，不全字段覆盖）
    let qualification = service
        .update_supplier_qualification_attachment_path(qualification_id, &attachment_url)
        .await?;

    tracing::info!(
        user_id = auth.user_id,
        supplier_id,
        qualification_id,
        attachment_path = %attachment_url,
        size_bytes = data.len(),
        "供应商资质附件上传成功"
    );
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(qualification).map_err(AppError::from)?,
        "资质附件上传成功",
    )))
}

/// GET /api/v1/erp/purchase/suppliers/{supplier_id}/qualifications/{qualification_id}/attachment
/// 资质附件受鉴权读取（fail-closed，不照抄头像匿名直读）：
/// - 未认证 → auth 中间件 401（AuthContext 提取器自身也会拒绝缺 extension 的请求）；
/// - 无 suppliers:read → permission 中间件 403（权限键由 URL 段推导，与资质列表同源）；
/// - 父供应商越出 data_scope / 资质双键错配 → handler 内门控拒绝；
/// - uploads/qualifications 目录无任何公开静态路由，文件字节只能经本端点读出；
/// - 磁盘定位仅使用服务端生成形态 `{supplier_id}_{qualification_id}.{ext}`，并对
///   canonicalize 结果做目录边界校验（防符号链接逃逸，对齐 routes/static.rs 头像服务防御）。
pub async fn get_supplier_qualification_attachment(
    Path((supplier_id, qualification_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let service = SupplierService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_supplier(supplier_id, Some(&data_scope_ctx))
        .await?;
    let qualification = service
        .get_supplier_qualification(supplier_id, qualification_id)
        .await?;
    let stored = qualification.attachment_path.clone().ok_or_else(|| {
        tracing::info!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            "读取资质附件：该资质尚未上传附件"
        );
        AppError::not_found("该资质尚未上传附件")
    })?;

    // 受控文件名解析：只取 URL 最后一段，且必须等于本记录的服务端生成形态；
    // 复用静态路由的防遍历消毒（拒绝 .. / 反斜杠 / 绝对段），任何漂移视为文件失效。
    let file_name = stored.rsplit('/').next().unwrap_or_default().to_string();
    let expected_prefix = format!("{}_{}.", supplier_id, qualification_id);
    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if !file_name.starts_with(&expected_prefix)
        || !QUALIFICATION_ATTACHMENT_ALLOWED_EXTS.contains(&ext.as_str())
        || crate::routes::static_routes::sanitize_static_path(&stored).is_none()
    {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            stored = %stored,
            "拒绝非法资质附件路径（疑似路径漂移/遍历），按文件不存在处理"
        );
        return Err(AppError::not_found("资质附件不存在或已失效，请重新上传"));
    }

    let dir = tokio::fs::canonicalize(QUALIFICATION_UPLOAD_DIR)
        .await
        .map_err(|_| AppError::not_found("资质附件文件不存在，可能已被清理，请重新上传"))?;
    let resolved = tokio::fs::canonicalize(dir.join(&file_name))
        .await
        .map_err(|_| AppError::not_found("资质附件文件不存在，可能已被清理，请重新上传"))?;
    if !resolved.starts_with(&dir) {
        tracing::warn!(
            user_id = auth.user_id,
            supplier_id,
            qualification_id,
            "拒绝符号链接越界访问资质附件"
        );
        return Err(AppError::not_found("资质附件不存在或已失效，请重新上传"));
    }

    let content_type: &'static str = match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        _ => "image/jpeg",
    };
    let content = tokio::fs::read(&resolved)
        .await
        .map_err(|e| AppError::internal(format!("资质附件读取失败：{}", e)))?;
    let size_bytes = content.len();

    let mut res = (StatusCode::OK, Body::from(content)).into_response();
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(content_type),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("inline; filename=\"{}\"", file_name))
            .map_err(|_| AppError::internal("构造附件响应头失败".to_string()))?,
    );
    // 商业敏感证照：禁止任何缓存（浏览器与共享代理均不落盘）
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    tracing::info!(
        user_id = auth.user_id,
        supplier_id,
        qualification_id,
        bytes = size_bytes,
        "资质附件读取成功（鉴权通过）"
    );
    Ok(res)
}
