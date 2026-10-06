use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::sales_price;
use crate::models::status::price_approval;
use crate::services::sales_price_service::{
    CreateSalesPriceInput, SalesPriceService, SalesPriceView, UpdateSalesPriceInput,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
// V15 P0-S12/P0-S15 修复（Batch 475d）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};
use validator::Validate;

// V15 P0-S12 修复（Batch 475d）：派生 Clone，export_prices 需要 clone 后覆盖分页参数用于全量导出
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Clone, Deserialize)]
pub struct SalesPriceQuery {
    pub product_id: Option<i32>,
    /// 客户等值筛选：前端筛选栏（SalesPriceFilter.vue 客户下拉）一直在传，此前本结构无键、
    /// serde 静默忽略 ⇒ 假筛选（#206/#160 同族），本轮接收并下推 service 谓词。
    pub customer_id: Option<i32>,
    /// 关键词筛选：语义 =「产品名称/客户名称」模糊匹配（筛选栏 placeholderKeyword 承诺），
    /// 经 LeftJoin 下推（多对一，不倍增行）。
    pub keyword: Option<String>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    /// 审批通过理由：本域服务端必填（缺失/空串/纯空白一律拒绝）。字段保持
    /// `Option<String>` 是为了让"缺键"调用走到统一的 `AppError` 校验信封，
    /// 而不是在 axum 解码层退化成无信封裸 400；必填语义在 handler 收口。
    pub approval_reason: Option<String>,
}

/// 审批拒绝入参：拒绝理由全域必填（trim 非空）并落 `rejected_reason` 列。
#[derive(Debug, Deserialize)]
pub struct RejectPriceRequest {
    pub reason: String,
}

/// 销售价目列表 `status` 筛选取值域：**分表钉**，等于权威词表 `price_approval` 去掉
/// `inactive`（销售侧无 inactive 写入方，DB CHECK `chk_sales_price_status` 即此三值
/// 集合 pending/approved/rejected——rejected 由扩集后继迁移 m0080 纳入，白名单同批补齐）。
/// 绝不与采购侧取并集——并集会让 `inactive` 在销售侧回到"合法但恒空"的假筛选老路。
const SALES_PRICE_STATUS_FILTER_ALLOWED: &[&str] = &[
    price_approval::PENDING,
    price_approval::APPROVED,
    price_approval::REJECTED,
];

/// 销售价目列表 `status` 筛选入参校验。
///
/// 此前该参数原样下推成 SQL 等值条件，越界值恒零命中并静默返回 200 + 空列表，把拼写
/// 错误伪装成"没有数据"。现按取值域拒绝并回显允许值；`None` 或去空白后的空串视为不加
/// 筛选（trim 语义与 greige_fabric / inventory_stock 同族先例一致，空串另有
/// `normalize_empty_query_params` 中间件在链路更外层先行剔除）。
fn validate_sales_price_status_param(raw: Option<&str>) -> Result<(), AppError> {
    let Some(value) = raw.map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    if SALES_PRICE_STATUS_FILTER_ALLOWED.contains(&value) {
        return Ok(());
    }
    Err(AppError::validation_displayable(format!(
        "销售价格状态筛选值 {value} 不是合法取值，允许值：{}",
        SALES_PRICE_STATUS_FILTER_ALLOWED.join("/")
    )))
}

pub async fn list_prices(
    Query(params): Query<SalesPriceQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<SalesPriceView>>>, AppError> {
    validate_sales_price_status_param(params.status.as_deref())?;

    info!("用户 {} 正在查询销售价格列表", auth.user_id);

    let service = SalesPriceService::new(state.db.clone());
    let query_params = crate::services::sales_price_service::SalesPriceQueryParams {
        product_id: params.product_id,
        customer_id: params.customer_id,
        keyword: params.keyword,
        customer_type: params.customer_type,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (prices, _total) = service.get_prices_list(query_params).await?;
    info!("销售价格列表查询成功，共 {} 条记录", prices.len());

    Ok(Json(ApiResponse::success(prices)))
}

pub async fn get_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<sales_price::Model>>, AppError> {
    info!("用户 {} 正在查询销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    let price = service.get_price(id).await?;
    info!("销售价格查询成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn create_price(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesPriceInput>,
) -> Result<Json<ApiResponse<sales_price::Model>>, AppError> {
    req.validate()?;

    info!(
        "用户 {} 正在创建销售价格，产品 ID: {}",
        auth.user_id, req.product_id
    );

    let service = SalesPriceService::new(state.db.clone());
    let price = service.create_price(req, auth.user_id).await?;
    info!("销售价格创建成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn approve_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    payload: OptionalJson<ApprovePriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 入参形态用 `OptionalJson`（utils::optional_json 语义表）：`Option<Json<T>>`
    // 是假可选——axum 0.8.9 只在完全不带 Content-Type 时才放行，"带 JSON 头 + 空体"
    // 仍被解码层判 400；缺体在此归一为 None，由下方"通过理由必填"校验分支给出
    // 统一 AppError 信封（字段必填语义不变，不属于放松必填）。
    let req = payload.0;

    // 端点单一职责：本端点只处理批准，approved=false 不做任何写入，拒绝指向真实
    // 存在的独立拒绝端点（reject_price）。
    if matches!(&req, Some(r) if !r.approved) {
        warn!(
            "用户 {} 批准销售价格被拒：记录 ID {id} 提交 approved=false（ID 只进日志不进文案）",
            auth.user_id
        );
        return Err(AppError::validation_displayable(
            "审批拒绝请提交至价格拒绝接口，本接口仅处理批准操作",
        ));
    }

    // 通过理由本域必填：缺失/空串/纯空白一律 400 VALIDATION_ERROR；可外显文案
    // 定性、不含记录 ID（utils/error.rs 安全边界），ID 只进 warn 日志。
    let approval_reason = req
        .and_then(|r| r.approval_reason)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            warn!(
                "用户 {} 批准销售价格被拒：记录 ID {id} 缺少审批通过理由（ID 只进日志不进文案）",
                auth.user_id
            );
            AppError::validation_displayable("审批通过理由不能为空")
        })?;

    info!(
        "用户 {} 正在批准销售价格，ID: {}，通过理由: {}",
        auth.user_id, id, approval_reason
    );

    let service = SalesPriceService::new(state.db.clone());
    service
        .approve_price(id, auth.user_id, approval_reason)
        .await?;
    info!("销售价格批准成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn reject_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<RejectPriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 拒绝理由必填样板照 quotation_handler.rs:253-257：trim 非空，空串/纯空白
    // 400 VALIDATION_ERROR；落库为 trim 后的值；可外显文案不含记录 ID。
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        warn!(
            "用户 {} 拒绝销售价格被拒：记录 ID {id} 拒绝理由为空（ID 只进日志不进文案）",
            auth.user_id
        );
        return Err(AppError::validation_displayable("审批拒绝理由不能为空"));
    }

    info!(
        "用户 {} 正在拒绝销售价格，ID: {}，拒绝理由: {}",
        auth.user_id, id, reason
    );

    let service = SalesPriceService::new(state.db.clone());
    service.reject_price(id, auth.user_id, reason).await?;
    info!("销售价格拒绝成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn get_price_history(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_price::Model>>>, AppError> {
    info!(
        "用户 {} 正在查询产品 {} 的价格历史",
        auth.user_id, product_id
    );

    let service = SalesPriceService::new(state.db.clone());
    let history = service.get_price_history(product_id).await?;
    info!("价格历史查询成功，共 {} 条记录", history.len());

    Ok(Json(ApiResponse::success(history)))
}

pub async fn update_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdateSalesPriceInput>,
) -> Result<Json<ApiResponse<sales_price::Model>>, AppError> {
    info!("用户 {} 正在更新销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    let price = service.update_price(id, req).await?;
    info!("销售价格更新成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn delete_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在删除销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    // 批次 94 P2-10：传入真实操作人 user_id 用于审计日志
    service.delete_price(id, auth.user_id).await?;
    info!("销售价格删除成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/erp/sales-prices/export - 导出销售价格列表（带水印 + 异步审计日志）；销售价格导出表头（14 列）
fn price_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "产品ID".to_string(),
        "客户ID".to_string(),
        "客户类型".to_string(),
        "价格".to_string(),
        "币种".to_string(),
        "单位".to_string(),
        "最小订购量".to_string(),
        "价格类型".to_string(),
        "价格等级".to_string(),
        "生效日期".to_string(),
        "到期日期".to_string(),
        "状态".to_string(),
        "创建时间".to_string(),
    ]
}

/// 从销售价格 JSON 对象构建 xlsx 行
fn build_price_row(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let get_str = |key: &str| -> String {
        obj.get(key)
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
        get_str("product_id"),
        get_str("customer_id"),
        get_str("customer_type"),
        get_str("price"),
        get_str("currency"),
        get_str("unit"),
        get_str("min_order_qty"),
        get_str("price_type"),
        get_str("price_level"),
        get_str("effective_date"),
        get_str("expiry_date"),
        get_str("status"),
        get_str("created_at"),
    ]
}

/// 构造销售价格列表 xlsx 表格
fn build_prices_table(prices_json: Vec<serde_json::Value>) -> Result<XlsxTable, AppError> {
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(prices_json.len());
    for p in prices_json {
        let obj = p
            .as_object()
            .ok_or_else(|| AppError::internal("销售价格序列化失败：期望 JSON 对象"))?;
        rows.push(build_price_row(obj));
    }
    Ok(XlsxTable {
        sheet_name: "销售价格列表".to_string(),
        headers: price_export_headers(),
        rows,
    })
}

/// 异步记录销售价格导出操作（审计自身）
fn record_prices_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    filename: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("sales_price".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出销售价格列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/sales-prices/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

/// V15 P0-S12 修复（Batch 475d）：导出接入后端 - 注入水印（operator/exported_at/extra 含条数） - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_prices_list 取全量数据（page=1/page_size=10000） - 不复用 list_prices handler 逻辑（保持单一职责）
pub async fn export_prices(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<SalesPriceQuery>,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：校验审批令牌（在 query 被 move 前提取）
    let download_token = query.download_token.clone();
    let approval =
        crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
            .enforce_export_download(download_token.as_deref(), "price_list")
            .await?;

    // 导出与列表同一筛选口径：越界 status 在列表已 400 化（C-2/R3），导出此前不收口
    // 仍会静默产出空表；同族缺陷同批修，校验放在审批令牌之后避免向未授权方外显。
    validate_sales_price_status_param(query.status.as_deref())?;

    let service = SalesPriceService::new(state.db.clone());

    // V15 P0-S12 修复（Batch 475d）：导出全量数据
    let query_params = crate::services::sales_price_service::SalesPriceQueryParams {
        product_id: query.product_id,
        customer_id: query.customer_id,
        keyword: query.keyword,
        customer_type: query.customer_type,
        status: query.status,
        page: 1,
        page_size: 10000,
    };

    let (prices, _total) = service.get_prices_list(query_params).await?;
    let row_count = prices.len();

    // 序列化为 JSON 以统一字段处理
    let prices_json: Vec<serde_json::Value> = prices
        .into_iter()
        .map(|p| serde_json::to_value(p).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let table = build_prices_table(prices_json)?;
    let filename = format!(
        "sales_prices_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_prices_export_audit(&state, &auth, row_count, &filename);

    // 敏感导出 fail-closed：记录令牌消费
    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(
            approval.id,
            filename.clone(),
            row_count as i64,
            String::new(),
        )
        .await;

    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("销售价格导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
