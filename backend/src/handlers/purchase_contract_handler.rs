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
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use tracing::info;
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
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateContractRequestDto {
    pub contract_no: String,
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: chrono::NaiveDate,
    pub remark: Option<String>,
}

/// P1-2m 修复（批次 81 v1 复审）：更新采购合同请求 DTO
/// 替代 update_contract 中的 Json<serde_json::Value>，提供强类型校验
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateContractDto {
    /// 合同名称：可选
    #[validate(length(max = 200, message = "合同名称长度不能超过200字符"))]
    pub contract_name: Option<String>,
    /// 付款条款：可选
    pub payment_terms: Option<String>,
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
        payment_terms: req.payment_terms,
        delivery_date: req.delivery_date,
        remark: req.remark,
    };

    let contract = service.create(create_req, auth.user_id).await?;
    info!("采购合同创建成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 审核合同
pub async fn approve_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审核采购合同 {}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    service.approve(id, auth.user_id).await?;

    let message = format!("合同 {} 审核成功", id);
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

    let service = PurchaseContractService::new(state.db.clone());

    // 获取现有合同
    let mut contract = service.get_by_id(id).await?;

    // 检查状态
    if contract.status != crate::models::status::contract::DRAFT {
        return Err(AppError::validation(
            "只有草稿状态的合同才能修改".to_string(),
        ));
    }

    // 更新字段
    if let Some(name) = req.contract_name {
        contract.contract_name = name;
    }
    if let Some(terms) = req.payment_terms {
        contract.payment_terms = Some(terms);
    }

    // 保存更新
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::purchase_contract::ActiveModel = contract.into();
    active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    let updated = active_model.update(&*state.db).await?;

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
        return Err(AppError::validation(
            "只有草稿状态的合同才能删除".to_string(),
        ));
    }

    // 软删除
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::purchase_contract::ActiveModel = contract.into();
    active_model.status = sea_orm::Set("cancelled".to_string());
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
