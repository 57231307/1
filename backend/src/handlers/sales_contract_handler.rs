use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::sales_contract;
use crate::services::sales_contract_service::{
    CreateContractItemRequest, CreateSalesContractRequest, ExecuteSalesContractRequest,
    SalesContractService,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
// V15 P0-S12/P0-S15 修复（Batch 475d）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// 销售合同查询参数 DTO
// V15 P0-S12 修复（Batch 475d）：派生 Clone，export_contracts 需要 clone 后覆盖分页参数用于全量导出
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, Deserialize)]
pub struct SalesContractQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建销售合同请求 DTO
///
/// P0 契约修复（本轮）：
/// - `delivery_date` 改 Option：真实列 sales_contracts.delivery_date 可空，原非 Option
///   导致前端未填时反序列化失败 → 400「参数错误」。
/// - 补齐表头真实列 signed_date/effective_date/expiry_date/payment_method/delivery_location。
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateSalesContractRequestDto {
    pub contract_no: String,
    pub contract_name: String,
    pub customer_id: i32,
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
    /// 合同明细行
    #[validate(nested)]
    pub items: Option<Vec<CreateContractItemDto>>,
}

/// 创建合同明细行 DTO
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateContractItemDto {
    pub product_id: Option<i32>,
    pub product_name: String,
    pub product_spec: Option<String>,
    pub unit: String,
    pub quantity: rust_decimal::Decimal,
    /// 交货数量允收容差（百分比，可空）：NULL 走默认解析；含「约」订单可写 10.00 覆盖。
    /// 范围校验 [0, 100]：`None` 合法（不覆盖），`Some(v)` 时校验 `0 <= v <= 100`。
    #[validate(custom(function = "validate_quantity_tolerance_pct"))]
    pub quantity_tolerance_pct: Option<rust_decimal::Decimal>,
    pub unit_price: rust_decimal::Decimal,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
}

/// 合同行交货允差百分比范围校验：Some 时必须在 [0, 100] 区间内。
/// validator 框架对 `Option<T>` 自动解包，`None` 时跳过不校验。
/// 上下限复用 [`crate::utils::delivery_tolerance`] 的共享单一真源常量，与其它域一致。
fn validate_quantity_tolerance_pct(
    value: &rust_decimal::Decimal,
) -> Result<(), validator::ValidationError> {
    use crate::utils::delivery_tolerance::{TOLERANCE_PCT_MAX, TOLERANCE_PCT_MIN};
    if *value < TOLERANCE_PCT_MIN || *value > TOLERANCE_PCT_MAX {
        return Err(validator::ValidationError::new(
            "合同行交货允差百分比(quantity_tolerance_pct)必须在0~100之间",
        ));
    }
    Ok(())
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 handlers/purchase_contract_handler.rs 中同名适配器形状一致（本批授权文件仅限
/// 合同/部门，各域 handler 内私有定义；跨域合并到共享工具需动 utils，超出本批授权范围）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// P1-2o 修复（批次 81 v1 复审）：更新销售合同请求 DTO
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖；items=明细整表替换。
/// NOT NULL 列（contract_name/customer_id，m0011 DDL）不开 null 清空，显式 null 由 service 拒绝。
/// 不含 contract_no：合同编号属系统生成单据号（前端预生成 + /document-no/check 查重，
/// 数据库 UNIQUE 兜底），编辑链路不接受改写；请求里携带的 contract_no 一律忽略。
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateSalesContractDto {
    /// 合同名称：NOT NULL 列——显式 null 被 service 拒绝（业务错误，非脱敏）
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(max = 200, message = "合同名称长度不能超过200字符"))]
    pub contract_name: Option<Option<String>>,
    /// 客户ID：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub customer_id: Option<Option<i32>>,
    /// 合同金额：DB 可空列 total_amount DECIMAL(15,2)（m0011 DDL 核实）
    #[serde(default, deserialize_with = "double_option")]
    pub total_amount: Option<Option<rust_decimal::Decimal>>,
    /// 合同类型：DB 可空列 contract_type VARCHAR(50)（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub contract_type: Option<Option<String>>,
    /// 付款条款：DB 可空列 payment_terms TEXT（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub payment_terms: Option<Option<String>>,
    /// 交货日期：DB 可空列 delivery_date DATE（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub delivery_date: Option<Option<chrono::NaiveDate>>,
    /// 签订日期：DB 可空列 signed_date DATE（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub signed_date: Option<Option<chrono::NaiveDate>>,
    /// 生效日期：DB 可空列 effective_date DATE（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub effective_date: Option<Option<chrono::NaiveDate>>,
    /// 到期日期：DB 可空列 expiry_date DATE（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub expiry_date: Option<Option<chrono::NaiveDate>>,
    /// 付款方式：DB 可空列 payment_method VARCHAR(50)（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub payment_method: Option<Option<String>>,
    /// 交货地点：DB 可空列 delivery_location VARCHAR(200)（m0011）
    #[serde(default, deserialize_with = "double_option")]
    pub delivery_location: Option<Option<String>>,
    /// 备注：DB 可空列 remark TEXT（m0016 补列）
    #[serde(default, deserialize_with = "double_option")]
    pub remark: Option<Option<String>>,
    /// 明细行（含 quantity_tolerance_pct 的 [0,100] 行级校验，与 create 同口径）。
    /// 数组字段不开放"显式 null 清全表"：清空明细须传空数组 `[]`，与"键缺席=不动明细"区分。
    #[validate(nested)]
    pub items: Option<Vec<CreateContractItemDto>>,
}

fn map_item_d_tos(
    items: Option<Vec<CreateContractItemDto>>,
) -> Option<Vec<CreateContractItemRequest>> {
    items.map(|items| {
        items
            .into_iter()
            .map(|item| CreateContractItemRequest {
                product_id: item.product_id,
                product_name: item.product_name,
                product_spec: item.product_spec,
                unit: item.unit,
                quantity: item.quantity,
                quantity_tolerance_pct: item.quantity_tolerance_pct,
                unit_price: item.unit_price,
                delivery_date: item.delivery_date,
                remarks: item.remarks,
            })
            .collect()
    })
}

/// 合同执行请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ExecuteSalesContractRequestDto {
    pub execution_type: String,
    pub execution_amount: rust_decimal::Decimal,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

/// 取消合同请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CancelSalesContractRequest {
    pub reason: String,
}

/// 获取销售合同列表
pub async fn list_contracts(
    Query(params): Query<SalesContractQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    info!("用户 {} 正在查询销售合同列表", auth.user_id);

    let service = SalesContractService::new(state.db.clone());
    let query_params = crate::services::sales_contract_service::SalesContractQueryParams {
        keyword: params.keyword,
        status: params.status,
        customer_id: params.customer_id,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (contracts, _total) = service.get_list(query_params).await?;
    info!("销售合同列表查询成功，共 {} 条记录", contracts.len());

    // created_by_name 富化：单次批量查询 users.real_name（杜绝逐行查询），
    // 悬挂外键仅让该列名为空、不丢行。出参键 = sales_contract 实体列 + created_by_name。
    let created_by_ids: Vec<i32> = contracts.iter().map(|c| c.created_by).collect();
    let name_map: std::collections::HashMap<i32, Option<String>> =
        crate::models::user::Entity::find()
            .filter(crate::models::user::Column::Id.is_in(created_by_ids))
            .all(&*state.db)
            .await?
            .into_iter()
            .map(|u| (u.id, u.real_name))
            .collect();

    let rows: Vec<serde_json::Value> = contracts
        .into_iter()
        .map(|c| {
            let created_by_name = name_map.get(&c.created_by).and_then(|n| n.clone());
            let mut value = serde_json::to_value(c).map_err(AppError::from)?;
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "created_by_name".to_string(),
                    created_by_name
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(ApiResponse::success(rows)))
}

/// 获取销售合同详情
pub async fn get_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<sales_contract::Model>>, AppError> {
    info!("用户 {} 正在查询销售合同详情：{}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    let contract = service.get_by_id(id).await?;
    info!("销售合同详情查询成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 获取销售合同明细行
pub async fn get_contract_items(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crate::models::sales_contract_item::Model>>>, AppError> {
    info!("用户 {} 正在查询销售合同 {} 的明细行", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    let items = service.get_items(id).await?;
    info!("销售合同 {} 明细行查询成功，共 {} 条", id, items.len());

    Ok(Json(ApiResponse::success(items)))
}

/// 创建销售合同
#[axum::debug_handler]
pub async fn create_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesContractRequestDto>,
) -> Result<Json<ApiResponse<sales_contract::Model>>, AppError> {
    info!(
        "用户 {} 正在创建销售合同：{}",
        auth.user_id, req.contract_no
    );

    // 合同行交货允差百分比范围校验（validator derive 范式，与 so/po create 同口径）：
    // 通过 `#[validate(nested)]` + 字段级 `custom` 校验 Some 值须落在 [0, 100]，None 跳过；
    // 判定语义与原内联循环完全一致，仅统一校验范式与上下限单一真源。
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = SalesContractService::new(state.db.clone());
    let create_req = CreateSalesContractRequest {
        contract_no: req.contract_no,
        contract_name: req.contract_name,
        customer_id: req.customer_id,
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
        items: map_item_d_tos(req.items),
    };

    let contract = service.create(create_req, auth.user_id).await?;
    info!("销售合同创建成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 审核销售合同
pub async fn approve_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审核销售合同 {}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    service.approve(id, auth.user_id).await?;

    let message = format!("合同 {} 审核成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 执行销售合同
#[axum::debug_handler]
pub async fn execute_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExecuteSalesContractRequestDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在执行销售合同 {}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    let execute_req = ExecuteSalesContractRequest {
        execution_type: req.execution_type,
        execution_amount: req.execution_amount,
        related_bill_type: req.related_bill_type,
        related_bill_id: req.related_bill_id,
        remark: req.remark,
    };

    service.execute(id, execute_req, auth.user_id).await?;

    let message = format!("合同 {} 执行成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 取消销售合同
#[axum::debug_handler]
pub async fn cancel_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelSalesContractRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在取消销售合同 {}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    service.cancel(id, auth.user_id, req.reason).await?;

    let message = format!("合同 {} 取消成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// PUT /api/v1/erp/sales-contracts/:id - 更新销售合同
pub async fn update_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateSalesContractDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新销售合同: ID={}", auth.username, id);

    // P1-2o 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    // P0 契约修复（本轮）：原实现在 handler 内联「取模型→改 2 个字段→保存」，
    // 无事务/无行锁且其余表头字段与明细全部丢失。改为下沉 service.update
    // （txn + lock_exclusive + DRAFT 状态门 + 表头全集 + 明细整表替换）。
    let service = SalesContractService::new(state.db.clone());
    let update_req = crate::services::sales_contract_service::UpdateSalesContractRequest {
        contract_name: req.contract_name,
        customer_id: req.customer_id,
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
        items: map_item_d_tos(req.items),
    };

    let updated = service.update(id, update_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated)?,
        "销售合同更新成功",
    )))
}

/// DELETE /api/v1/erp/sales-contracts/:id - 删除销售合同
pub async fn delete_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除销售合同: ID={}", auth.username, id);

    let service = SalesContractService::new(state.db.clone());

    // 获取现有合同
    let contract = service.get_by_id(id).await?;

    // 检查状态
    if contract.status != crate::models::status::contract::DRAFT {
        // 状态门：合同非草稿，删除前置未满足，归业务族；文案纯规则可外显
        return Err(AppError::business_displayable(
            "只有草稿状态的合同才能删除".to_string(),
        ));
    }

    // 软删除
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::sales_contract::ActiveModel = contract.into();
    active_model.status = sea_orm::Set("cancelled".to_string());
    active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    active_model.update(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "销售合同已删除",
    )))
}

/// 销售合同导出表头（13 列）
fn contracts_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "合同编号".to_string(),
        "合同名称".to_string(),
        "合同类型".to_string(),
        "客户ID".to_string(),
        "客户名称".to_string(),
        "总金额".to_string(),
        "签订日期".to_string(),
        "生效日期".to_string(),
        "到期日期".to_string(),
        "付款条款".to_string(),
        "状态".to_string(),
        "创建时间".to_string(),
    ]
}

/// 从 serde_json::Value 提取销售合同行数据
fn build_contract_row(c: &serde_json::Value) -> Vec<String> {
    let get_str = |key: &str| -> String {
        c.get(key)
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
        get_str("contract_no"),
        get_str("contract_name"),
        get_str("contract_type"),
        get_str("customer_id"),
        get_str("customer_name"),
        get_str("total_amount"),
        get_str("signed_date"),
        get_str("effective_date"),
        get_str("expiry_date"),
        get_str("payment_terms"),
        get_str("status"),
        get_str("created_at"),
    ]
}

/// 构造销售合同列表 xlsx 表格
fn build_contracts_table(contracts_json: &[serde_json::Value]) -> XlsxTable {
    XlsxTable {
        sheet_name: "销售合同列表".to_string(),
        headers: contracts_export_headers(),
        rows: contracts_json.iter().map(build_contract_row).collect(),
    }
}

/// 异步记录销售合同导出操作（审计自身，best-effort 不阻塞响应）
fn record_contracts_export_audit(
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
        resource_type: Some("sales_contract".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出销售合同列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/sales-contracts/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    svc.record_async(event, None);
}

/// GET /api/v1/erp/sales-contracts/export - 导出销售合同列表（带水印 + 异步审计日志）；V15 P0-S12 修复（Batch 475d）：导出接入后端 - 注入水印（operator/exported_at/extra
/// 含条数） - 异步审计日志（OperationType::Export） - 直接调 service.get_list 取全量数据（page=1/page_size=10000） - 不复用 list_contracts handler 逻辑（保持单一职责）
pub async fn export_contracts(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<SalesContractQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = SalesContractService::new(state.db.clone());
    // V15 P0-S12 修复（Batch 475d）：导出全量数据（覆盖分页参数）
    let query_params = crate::services::sales_contract_service::SalesContractQueryParams {
        keyword: query.keyword,
        status: query.status,
        customer_id: query.customer_id,
        page: 1,
        page_size: 10000,
    };
    let (contracts, _total) = service.get_list(query_params).await?;
    let row_count = contracts.len();
    // 序列化为 JSON 以统一字段处理
    let contracts_json: Vec<serde_json::Value> = contracts
        .into_iter()
        .map(|c| serde_json::to_value(c).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;
    let table = build_contracts_table(&contracts_json);
    let filename = format!(
        "sales_contracts_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_contracts_export_audit(&state, &auth, row_count, &filename);
    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("销售合同导出（共 {} 条）", row_count)),
    };
    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
