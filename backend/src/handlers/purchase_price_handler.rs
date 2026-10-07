use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::purchase_price;
use crate::models::status::price_approval;
use crate::services::purchase_price_service::{
    CreatePurchasePriceInput, PurchasePriceService, PurchasePriceView,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use tracing::{info, warn};
use validator::Validate;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct PurchasePriceQuery {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
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

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdatePriceRequest {
    pub price: String,
    pub expiry_date: Option<String>,
    pub status: Option<String>,
}

/// 采购价目列表 `status` 筛选入参校验（取值域 = `price_approval::ALL`，与 DB CHECK
/// `chk_purchase_price_status` 全等，词表单源不写字符串字面量）。
///
/// 越界值按取值域 `price_approval::ALL` 拒绝并回显允许值；`None` 或去空白后的空串视为不加
/// 筛选（trim 语义与 greige_fabric / inventory_stock 一致，空串另有 `normalize_empty_query_params`
/// 中间件在链路更外层先行剔除）。
fn validate_purchase_price_status_param(raw: Option<&str>) -> Result<(), AppError> {
    let Some(value) = raw.map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    if price_approval::ALL.contains(&value) {
        return Ok(());
    }
    Err(AppError::validation_displayable(format!(
        "采购价格状态筛选值 {value} 不是合法取值，允许值：{}",
        price_approval::ALL.join("/")
    )))
}

pub async fn list_prices(
    Query(params): Query<PurchasePriceQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PurchasePriceView>>>, AppError> {
    validate_purchase_price_status_param(params.status.as_deref())?;

    info!("用户 {} 正在查询采购价格列表", auth.user_id);

    let service = PurchasePriceService::new(state.db.clone());
    let query_params = crate::services::purchase_price_service::PurchasePriceQueryParams {
        product_id: params.product_id,
        supplier_id: params.supplier_id,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (prices, _total) = service.get_prices_list(query_params).await?;
    info!("采购价格列表查询成功，共 {} 条记录", prices.len());

    Ok(Json(ApiResponse::success(prices)))
}

pub async fn get_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<purchase_price::Model>>, AppError> {
    info!("用户 {} 正在查询采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(state.db.clone());
    let price = service.get_price(id).await?;
    info!("采购价格查询成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn create_price(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePurchasePriceInput>,
) -> Result<Json<ApiResponse<purchase_price::Model>>, AppError> {
    req.validate()?;

    info!(
        "用户 {} 正在创建采购价格，产品 ID: {}",
        auth.user_id, req.product_id
    );

    let service = PurchasePriceService::new(state.db.clone());
    let price = service.create_price(req, auth.user_id).await?;
    info!("采购价格创建成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn update_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdatePriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在更新采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(state.db.clone());
    service
        .update_price(
            id,
            req.price
                .parse()
                .map_err(|e| AppError::validation_displayable(format!("价格格式错误：{}", e)))?,
            req.expiry_date,
            req.status,
        )
        .await?;
    info!("采购价格更新成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn delete_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在删除采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(state.db.clone());
    service.delete_price(id).await?;
    info!("采购价格删除成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
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
            "用户 {} 批准采购价格被拒：记录 ID {id} 提交 approved=false（ID 只进日志不进文案）",
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
                "用户 {} 批准采购价格被拒：记录 ID {id} 缺少审批通过理由（ID 只进日志不进文案）",
                auth.user_id
            );
            AppError::validation_displayable("审批通过理由不能为空")
        })?;

    info!(
        "用户 {} 正在批准采购价格，ID: {}，通过理由: {}",
        auth.user_id, id, approval_reason
    );

    let service = PurchasePriceService::new(state.db.clone());
    service
        .approve_price(id, auth.user_id, approval_reason)
        .await?;
    info!("采购价格批准成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn reject_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<RejectPriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 拒绝理由必填与报价单域 reject 通道同范式（quotation_handler）：trim 非空，空串/纯空白
    // 400 VALIDATION_ERROR；落库为 trim 后的值；可外显文案不含记录 ID。
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        warn!(
            "用户 {} 拒绝采购价格被拒：记录 ID {id} 拒绝理由为空（ID 只进日志不进文案）",
            auth.user_id
        );
        return Err(AppError::validation_displayable("审批拒绝理由不能为空"));
    }

    info!(
        "用户 {} 正在拒绝采购价格，ID: {}，拒绝理由: {}",
        auth.user_id, id, reason
    );

    let service = PurchasePriceService::new(state.db.clone());
    service.reject_price(id, auth.user_id, reason).await?;
    info!("采购价格拒绝成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn get_price_history(
    State(state): State<AppState>,
    Path(material_id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<purchase_price::Model>>>, AppError> {
    info!(
        "用户 {} 正在查询物料 {} 的价格历史",
        auth.user_id, material_id
    );

    let service = PurchasePriceService::new(state.db.clone());
    let history = service.get_price_history(material_id).await?;
    info!("价格历史查询成功，共 {} 条记录", history.len());

    Ok(Json(ApiResponse::success(history)))
}

/// 按产品 ID 查询采购价格历史
/// 前端调用: GET /purchase-prices/history/:product_id
pub async fn get_price_history_by_product(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<purchase_price::Model>>>, AppError> {
    info!("正在按产品 ID {} 查询采购价格历史", product_id);

    let service = PurchasePriceService::new(state.db.clone());
    let history = service.get_price_history(product_id).await?;
    info!("按产品 ID 的价格历史查询成功，共 {} 条记录", history.len());

    Ok(Json(ApiResponse::success(history)))
}

/// batch-13 P3: 价格清单导入请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ImportPriceRequest {
    pub prices: Vec<ImportPriceItem>,
}

/// batch-13 P3: 价格清单导入项
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ImportPriceItem {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: String,
    pub currency: Option<String>,
    pub unit: String,
    pub price_type: String,
    pub effective_date: Option<String>,
    pub expiry_date: Option<String>,
}

/// batch-13 P3: 价格清单导入响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct ImportPriceResponse {
    pub success_count: u32,
    pub failed_count: u32,
    pub errors: Vec<String>,
}

/// POST /api/v1/erp/purchase-prices/import - 价格清单导入
pub async fn import_prices(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<ImportPriceRequest>,
) -> Result<Json<ApiResponse<ImportPriceResponse>>, AppError> {
    info!(
        "用户 {} 正在导入价格清单，共 {} 条",
        auth.user_id,
        request.prices.len()
    );

    let service = PurchasePriceService::new(state.db.clone());
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for (index, item) in request.prices.iter().enumerate() {
        let price = match item.price.parse::<rust_decimal::Decimal>() {
            Ok(p) => p,
            Err(_) => {
                failed_count += 1;
                errors.push(format!("第 {} 条价格格式错误: {}", index + 1, item.price));
                continue;
            }
        };

        let input = CreatePurchasePriceInput {
            product_id: item.product_id,
            supplier_id: item.supplier_id,
            price,
            currency: item.currency.clone(),
            unit: item.unit.clone(),
            price_type: item.price_type.clone(),
            min_order_qty: None,
            effective_date: item.effective_date.clone(),
            expiry_date: item.expiry_date.clone(),
        };

        if let Err(e) = input.validate() {
            failed_count += 1;
            errors.push(format!("第 {} 条校验失败: {}", index + 1, e));
            continue;
        }

        match service.create_price(input, auth.user_id).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("第 {} 条导入失败: {}", index + 1, e));
            }
        }
    }

    info!(
        "价格清单导入完成: 成功 {}, 失败 {}",
        success_count, failed_count
    );

    Ok(Json(ApiResponse::success(ImportPriceResponse {
        success_count,
        failed_count,
        errors,
    })))
}
