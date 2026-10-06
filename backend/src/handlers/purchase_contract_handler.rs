use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::purchase_contract;
use crate::services::import_export_service::MAX_EXPORT_ROWS;
use crate::services::purchase_contract_service::{
    ContractQueryParams, CreateContractRequest, ExecuteContractRequest, PurchaseContractService,
    PurchaseContractView,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use validator::Validate;

/// 合同查询参数 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ContractQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    /// 签订日期范围（前端 date_range 数组：重复键 date_range=from&date_range=to）
    #[serde(default)]
    pub date_range: Option<Vec<String>>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建采购合同请求 DTO
///
/// P0 契约修复（本轮）：delivery_date 改 Option（真实列可空）；补齐真实列
/// contract_type/signed_date/effective_date/expiry_date/payment_method/delivery_location。
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateContractRequestDto {
    pub contract_no: String,
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub contract_type: Option<String>,
    pub payment_terms: Option<String>,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub signed_date: Option<chrono::NaiveDate>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub payment_method: Option<String>,
    pub delivery_location: Option<String>,
    pub remark: Option<String>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 utils/query_params.rs 的 empty_str_as_none 同属 serde 输入整形，非业务常量；
/// 本批授权文件仅限合同/部门，故各域 handler 内私有定义，后续可上收为共享工具。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// P1-2m 修复（批次 81 v1 复审）：更新采购合同请求 DTO
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖。
/// NOT NULL 列（contract_name/supplier_id，m0009 DDL）同样保持三态可辨，但显式 null
/// 由 service 层判为业务错误拒绝——不开 null 清空语义。
/// 不含 contract_no：合同编号属系统生成单据号（前端预生成 + /document-no/check 查重，
/// 数据库 UNIQUE 兜底），编辑链路不接受改写；请求里携带的 contract_no 一律忽略。
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateContractDto {
    /// 合同名称：NOT NULL 列——显式 null 被 service 拒绝（业务错误，非脱敏）
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 200, message = "合同名称长度不能超过200字符"))]
    pub contract_name: Option<Option<String>>,
    /// 供应商ID：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub supplier_id: Option<Option<i32>>,
    /// 合同金额：DB 可空列 total_amount DECIMAL(15,2)（m0009 DDL 核实）
    #[serde(default, deserialize_with = "double_option")]
    pub total_amount: Option<Option<rust_decimal::Decimal>>,
    /// 合同类型：DB 可空列 contract_type VARCHAR(50)（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub contract_type: Option<Option<String>>,
    /// 付款条款：DB 可空列 payment_terms TEXT（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub payment_terms: Option<Option<String>>,
    /// 交货日期：DB 可空列 delivery_date DATE（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub delivery_date: Option<Option<chrono::NaiveDate>>,
    /// 签订日期：DB 可空列 signed_date DATE（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub signed_date: Option<Option<chrono::NaiveDate>>,
    /// 生效日期：DB 可空列 effective_date DATE（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub effective_date: Option<Option<chrono::NaiveDate>>,
    /// 到期日期：DB 可空列 expiry_date DATE（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub expiry_date: Option<Option<chrono::NaiveDate>>,
    /// 付款方式：DB 可空列 payment_method VARCHAR(50)（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub payment_method: Option<Option<String>>,
    /// 交货地点：DB 可空列 delivery_location VARCHAR(200)（m0009）
    #[serde(default, deserialize_with = "double_option")]
    pub delivery_location: Option<Option<String>>,
    /// 备注：DB 可空列 remark TEXT（m0016 补列）
    #[serde(default, deserialize_with = "double_option")]
    pub remark: Option<Option<String>>,
}

/// 合同执行请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ExecuteContractRequestDto {
    pub execution_type: String,
    pub execution_amount: rust_decimal::Decimal,
    pub execution_date: chrono::NaiveDate,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

/// 取消合同请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CancelContractRequest {
    pub reason: String,
}

/// 审批通过请求 DTO：通过理由本域必填（缺失/空串/纯空白一律服务端拒绝）。
/// 字段保持 `Option<String>` 是为了让"缺键/不带 body"的调用走到统一的 `AppError`
/// 校验信封，而不是在 axum 解码层退化成无信封裸 400；必填语义在 handler 收口。
#[derive(Debug, Deserialize)]
pub struct ApproveContractRequest {
    pub approval_reason: Option<String>,
}

/// 审批拒绝请求 DTO：拒绝理由全域必填（trim 非空）并落 `rejected_reason` 列。
#[derive(Debug, Deserialize)]
pub struct RejectContractRequest {
    pub reason: String,
}

/// 获取合同列表
pub async fn list_contracts(
    Query(params): Query<ContractQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PurchaseContractView>>>, AppError> {
    info!("用户 {} 正在查询采购合同列表", auth.user_id);

    let service = PurchaseContractService::new(state.db.clone());
    let query_params = crate::services::purchase_contract_service::ContractQueryParams {
        keyword: params.keyword,
        status: params.status,
        supplier_id: params.supplier_id,
        date_range: params.date_range,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (contracts, _total) = service.get_list(query_params).await?;
    info!("采购合同列表查询成功，共 {} 条记录", contracts.len());

    Ok(Json(ApiResponse::success(contracts)))
}

/// 获取合同详情
pub async fn get_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<purchase_contract::Model>>, AppError> {
    info!("用户 {} 正在查询采购合同详情：{}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    let contract = service.get_by_id(id).await?;
    info!("采购合同详情查询成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 创建合同
#[axum::debug_handler]
pub async fn create_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateContractRequestDto>,
) -> Result<Json<ApiResponse<purchase_contract::Model>>, AppError> {
    info!(
        "用户 {} 正在创建采购合同：{}",
        auth.user_id, req.contract_no
    );

    let service = PurchaseContractService::new(state.db.clone());
    let create_req = CreateContractRequest {
        contract_no: req.contract_no,
        contract_name: req.contract_name,
        supplier_id: req.supplier_id,
        total_amount: req.total_amount,
        contract_type: req.contract_type,
        payment_terms: req.payment_terms,
        delivery_date: req.delivery_date,
        signed_date: req.signed_date,
        effective_date: req.effective_date,
        expiry_date: req.expiry_date,
        payment_method: req.payment_method,
        delivery_location: req.delivery_location,
        remark: req.remark,
    };

    let contract = service.create(create_req, auth.user_id).await?;
    info!("采购合同创建成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 审核合同（通过动作）
// 审批拆为「通过 / 拒绝」两条动作：本端点只处理通过，通过理由必填并真实落
// `approval_reason` 列；拒绝动作在 /reject（理由落 `rejected_reason` 列）。
pub async fn approve_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    payload: OptionalJson<ApproveContractRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    // 入参形态用 `OptionalJson`（utils::optional_json 语义表）：`Option<Json<T>>`
    // 是假可选——axum 0.8.9 只在完全不带 Content-Type 时才放行，"带 JSON 头 + 空体"
    // 仍被解码层判 400；缺体在此归一为 None，由下方必填分支给出统一 AppError 信封。
    let approval_reason = payload
        .0
        .and_then(|r| r.approval_reason)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            // 可外显文案只定性规则、不带记录 ID（utils/error.rs 安全边界），ID 只进日志。
            warn!(
                "用户 {} 审核采购合同被拒：合同 ID {id} 缺少审批通过理由（ID 只进日志不进文案）",
                auth.user_id
            );
            AppError::validation_displayable("审批通过理由不能为空")
        })?;

    info!(
        "用户 {} 正在审核采购合同 {}，通过理由: {}",
        auth.user_id, id, approval_reason
    );

    let service = PurchaseContractService::new(state.db.clone());
    service.approve(id, auth.user_id, approval_reason).await?;

    let message = format!("合同 {} 审核成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 拒绝合同（拒绝动作）：draft → rejected 终态流转，拒绝理由落 `rejected_reason` 列
pub async fn reject_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RejectContractRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    // 拒绝理由服务端必填（trim 非空），落库为 trim 后的值——与通过理由同一收口口径；
    // 可外显文案定性、不带记录 ID。
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        warn!(
            "用户 {} 拒绝采购合同被拒：合同 ID {id} 拒绝理由为空（ID 只进日志不进文案）",
            auth.user_id
        );
        return Err(AppError::validation_displayable("审批拒绝理由不能为空"));
    }

    info!(
        "用户 {} 正在拒绝采购合同 {}，拒绝理由: {}",
        auth.user_id, id, reason
    );

    let service = PurchaseContractService::new(state.db.clone());
    service.reject(id, auth.user_id, reason).await?;

    let message = format!("合同 {} 已拒绝", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 执行合同
#[axum::debug_handler]
pub async fn execute_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExecuteContractRequestDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在执行采购合同 {}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    let execute_req = ExecuteContractRequest {
        execution_type: req.execution_type,
        execution_amount: req.execution_amount,
        execution_date: req.execution_date,
        related_bill_type: req.related_bill_type,
        related_bill_id: req.related_bill_id,
        remark: req.remark,
    };

    service.execute(id, execute_req, auth.user_id).await?;

    let message = format!("合同 {} 执行成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 取消合同
#[axum::debug_handler]
pub async fn cancel_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelContractRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在取消采购合同 {}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    service.cancel(id, auth.user_id, req.reason).await?;

    let message = format!("合同 {} 取消成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// PUT /api/v1/erp/purchase-contracts/:id - 更新合同
pub async fn update_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateContractDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新采购合同: ID={}", auth.username, id);

    // P1-2m 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    // P0 契约修复（本轮）：下沉 service.update（txn + lock_exclusive + DRAFT 状态门 + 表头全集）
    let service = PurchaseContractService::new(state.db.clone());
    let update_req = crate::services::purchase_contract_service::UpdateContractRequest {
        contract_name: req.contract_name,
        supplier_id: req.supplier_id,
        total_amount: req.total_amount,
        contract_type: req.contract_type,
        payment_terms: req.payment_terms,
        delivery_date: req.delivery_date,
        signed_date: req.signed_date,
        effective_date: req.effective_date,
        expiry_date: req.expiry_date,
        payment_method: req.payment_method,
        delivery_location: req.delivery_location,
        remark: req.remark,
    };

    let updated = service.update(id, update_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated)?,
        "采购合同更新成功",
    )))
}

/// DELETE /api/v1/erp/purchase-contracts/:id - 删除合同
pub async fn delete_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除采购合同: ID={}", auth.username, id);

    let service = PurchaseContractService::new(state.db.clone());

    // 获取现有合同
    let contract = service.get_by_id(id).await?;

    // 检查状态
    if contract.status != crate::models::status::contract::DRAFT {
        // 状态门：合同非草稿，删除前置未满足，归业务族；文案纯规则可外显
        return Err(AppError::business_displayable(
            "只有草稿状态的合同才能删除".to_string(),
        ));
    }

    // 软删除（写入值与权威词表 contract 同源，禁止裸字面量）
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::purchase_contract::ActiveModel = contract.into();
    active_model.status = sea_orm::Set(crate::models::status::contract::CANCELLED.to_string());
    active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    active_model.update(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "采购合同已删除",
    )))
}

/// GET /api/v1/erp/purchase/purchase-contracts/export - 导出采购合同列表为 xlsx
///
/// 复用 `list_contracts` 的查询参数（ContractQuery）与查询逻辑（PurchaseContractService::get_list，
/// 含 keyword/status/supplier_id/date_range 过滤与创建人姓名 LEFT JOIN），强制行数上限 MAX_EXPORT_ROWS
/// 防大表导出 OOM；导出真实列生成 OOXML xlsx 字节流，以 blob（application/vnd...sheet +
/// Content-Disposition: attachment）返回，前端用 URL.createObjectURL 触发下载。
pub async fn export_purchase_contracts(
    Query(params): Query<ContractQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    info!("用户 {} 正在导出采购合同", auth.username);

    let service = PurchaseContractService::new(state.db.clone());
    // 分页参数不参与导出：page 固定 1，page_size 取导出上限（与既有导出 MAX_EXPORT_ROWS 口径一致）
    let query_params = ContractQueryParams {
        keyword: params.keyword,
        status: params.status,
        supplier_id: params.supplier_id,
        date_range: params.date_range,
        page: 1,
        page_size: MAX_EXPORT_ROWS as i64,
    };

    let (contracts, _total) = service.get_list(query_params).await?;
    let row_count = contracts.len();

    let table = XlsxTable {
        sheet_name: "采购合同列表".to_string(),
        headers: purchase_contracts_export_headers(),
        rows: contracts.iter().map(build_purchase_contract_row).collect(),
    };

    let filename = format!(
        "purchase_contracts_export_{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("采购合同列表导出（共 {} 条）", row_count)),
    };

    record_purchase_contracts_export_audit(&state, &auth, row_count);

    info!("采购合同导出成功，共 {} 条", row_count);
    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}

/// 采购合同导出表头（16 列，均为 purchase_contracts 真实列 + 创建人 JOIN 名列）
fn purchase_contracts_export_headers() -> Vec<String> {
    vec![
        "合同编号".to_string(),
        "合同名称".to_string(),
        "合同类型".to_string(),
        "供应商名称".to_string(),
        "合同金额".to_string(),
        "签订日期".to_string(),
        "生效日期".to_string(),
        "到期日期".to_string(),
        "付款条款".to_string(),
        "付款方式".to_string(),
        "交货日期".to_string(),
        "交货地点".to_string(),
        "状态".to_string(),
        "创建人".to_string(),
        "创建时间".to_string(),
        "更新时间".to_string(),
    ]
}

/// Option<NaiveDate> 渲染为 `YYYY-MM-DD`，None 为空串
fn fmt_optional_date(d: Option<chrono::NaiveDate>) -> String {
    d.map(|x| x.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// DateTime<Utc> 渲染为本地可读时间串
fn fmt_datetime(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 从强类型读模型 PurchaseContractView 构造导出行（列顺序与表头一致）
fn build_purchase_contract_row(c: &PurchaseContractView) -> Vec<String> {
    vec![
        c.contract_no.clone(),
        c.contract_name.clone(),
        c.contract_type.clone().unwrap_or_default(),
        c.supplier_name.clone().unwrap_or_default(),
        c.total_amount.map(|d| d.to_string()).unwrap_or_default(),
        fmt_optional_date(c.signed_date),
        fmt_optional_date(c.effective_date),
        fmt_optional_date(c.expiry_date),
        c.payment_terms.clone().unwrap_or_default(),
        c.payment_method.clone().unwrap_or_default(),
        fmt_optional_date(c.delivery_date),
        c.delivery_location.clone().unwrap_or_default(),
        c.status.clone(),
        c.created_by_name.clone().unwrap_or_default(),
        fmt_datetime(c.created_at),
        fmt_datetime(c.updated_at),
    ]
}

/// 异步记录采购合同导出操作（审计自身，best-effort 不阻塞响应）
fn record_purchase_contracts_export_audit(state: &AppState, auth: &AuthContext, row_count: usize) {
    use crate::models::audit_log::{OperationType, Severity};
    use crate::services::audit_log_service::{AuditEvent, AuditLogService};
    use std::sync::Arc;

    let svc = AuditLogService::new(state.db.clone());
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("purchase-contract".to_string()),
        resource_id: None,
        resource_name: Some("采购合同列表导出".to_string()),
        description: Some(format!("导出 {} 条采购合同数据（含水印）", row_count)),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/purchase/purchase-contracts/export".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    Arc::new(svc).record_async(event, None);
}
