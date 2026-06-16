//! 面料多色号定价扩展 Handler
//!
//! 实现 13 个 HTTP 端点：色号价格 CRUD + 批量调价 + 审批 + 历史 + 计算 + 阶梯价 + 客户专属价 + 季节规则
//! 设计依据：docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4.3
//! 创建时间: 2026-06-18

use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::models::color_price_dto::{
    ApproveColorPriceDto, BatchAdjustPriceDto, ColorPriceDetail, ColorPriceListItem,
    CreateColorPriceDto, ListColorPricesQuery, PagedResponse, PriceCalcRequest, UpdateColorPriceDto,
};
use crate::models::color_price_history_dto::PriceHistoryItem;
use crate::models::color_price_tier_dto::CreatePriceTierDto;
use crate::models::customer_color_price_dto::{
    CreateCustomerColorPriceDto, ListCustomerColorPricesQuery,
};
use crate::models::seasonal_price_rule_dto::{
    CreateSeasonalRuleDto, ListSeasonalRulesQuery,
};
use crate::services::color_price_batch_service::{BatchError, ColorPriceBatchService};
use crate::services::color_price_crud_service::{ColorPriceCrudService, CrudError};
use crate::services::color_price_history_service::ColorPriceHistoryService;
use crate::services::color_price_seasonal_service::{ColorPriceSeasonalService, SeasonalError};
use crate::services::color_price_tier_service::{ColorPriceTierService, TierError};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ----------------------------------------------------------------------
// 错误转换辅助
// ----------------------------------------------------------------------

fn crud_err(e: CrudError) -> AppError {
    match e {
        CrudError::NotFound => AppError::not_found("色号价格不存在"),
        CrudError::InvalidState => AppError::business("当前状态不允许此操作"),
        CrudError::Validation(msg) => AppError::validation(msg),
        CrudError::Database(e) => AppError::database(e.to_string()),
    }
}

fn batch_err(e: BatchError) -> AppError {
    match e {
        BatchError::PriceNotFound(id) => AppError::not_found(format!("色号价格不存在: id={}", id)),
        BatchError::Validation(msg) => AppError::validation(msg),
        BatchError::Database(e) => AppError::database(e.to_string()),
    }
}

fn tier_err(e: TierError) -> AppError {
    match e {
        TierError::NotFound => AppError::not_found("阶梯价不存在"),
        TierError::PriceNotFound => AppError::not_found("色号价格不存在"),
        TierError::Validation(msg) => AppError::validation(msg),
        TierError::Database(e) => AppError::database(e.to_string()),
    }
}

fn seasonal_err(e: SeasonalError) -> AppError {
    match e {
        SeasonalError::NotFound => AppError::not_found("季节规则不存在"),
        SeasonalError::Validation(msg) => AppError::validation(msg),
        SeasonalError::Database(e) => AppError::database(e.to_string()),
    }
}

fn model_to_list_item(m: crate::models::product_color_price::Model) -> ColorPriceListItem {
    ColorPriceListItem {
        id: m.id,
        product_id: m.product_id,
        color_id: m.color_id,
        currency: m.currency,
        base_price: m.base_price,
        effective_from: m.effective_from,
        effective_to: m.effective_to,
        customer_level: m.customer_level,
        min_quantity: m.min_quantity,
        max_quantity: m.max_quantity,
        customer_id: m.customer_id,
        season: m.season,
        is_active: m.is_active,
        priority: m.priority,
        approval_status: m.approval_status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn model_to_detail(m: crate::models::product_color_price::Model) -> ColorPriceDetail {
    ColorPriceDetail {
        id: m.id,
        product_id: m.product_id,
        color_id: m.color_id,
        currency: m.currency,
        base_price: m.base_price,
        effective_from: m.effective_from,
        effective_to: m.effective_to,
        customer_level: m.customer_level,
        min_quantity: m.min_quantity,
        max_quantity: m.max_quantity,
        customer_id: m.customer_id,
        season: m.season,
        is_active: m.is_active,
        priority: m.priority,
        notes: m.notes,
        created_by: m.created_by,
        approved_by: m.approved_by,
        approved_at: m.approved_at,
        approval_status: m.approval_status,
        tenant_id: m.tenant_id,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

// ----------------------------------------------------------------------
// 色号价格 CRUD（5 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-prices - 色号价格列表
pub async fn list_color_prices(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListColorPricesQuery>,
) -> Result<Json<ApiResponse<PagedResponse<ColorPriceListItem>>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceCrudService::from_state(&state);

    let (items, total) = service.list(tenant_id, &query).await.map_err(crud_err)?;
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let list: Vec<ColorPriceListItem> = items.into_iter().map(model_to_list_item).collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: list,
        total,
        page,
        page_size,
    })))
}

/// POST /api/v1/erp/color-prices - 新建色号价格
pub async fn create_color_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateColorPriceDto>,
) -> Result<Json<ApiResponse<ColorPriceDetail>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let user_id = auth.user_id as i64;
    let service = ColorPriceCrudService::from_state(&state);

    let created = service.create(dto, tenant_id, user_id).await.map_err(crud_err)?;
    Ok(Json(ApiResponse::success(model_to_detail(created))))
}

/// GET /api/v1/erp/color-prices/:id - 色号价格详情
pub async fn get_color_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ColorPriceDetail>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceCrudService::from_state(&state);

    let m = service.get_by_id(id, tenant_id).await.map_err(crud_err)?;
    Ok(Json(ApiResponse::success(model_to_detail(m))))
}

/// PUT /api/v1/erp/color-prices/:id - 更新色号价格
pub async fn update_color_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateColorPriceDto>,
) -> Result<Json<ApiResponse<ColorPriceDetail>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceCrudService::from_state(&state);

    let m = service.update(id, tenant_id, dto).await.map_err(crud_err)?;
    Ok(Json(ApiResponse::success(model_to_detail(m))))
}

/// DELETE /api/v1/erp/color-prices/:id - 软删除
pub async fn delete_color_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ColorPriceDetail>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceCrudService::from_state(&state);

    let m = service.delete(id, tenant_id).await.map_err(crud_err)?;
    Ok(Json(ApiResponse::success(model_to_detail(m))))
}

// ----------------------------------------------------------------------
// 批量调价 / 审批（2 端点）
// ----------------------------------------------------------------------

/// POST /api/v1/erp/color-prices/batch-adjust - 批量调价
pub async fn batch_adjust_color_prices(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<BatchAdjustPriceDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let user_id = auth.user_id as i64;
    let service = ColorPriceBatchService::from_state(&state);

    let result = service
        .batch_adjust(dto, tenant_id, user_id)
        .await
        .map_err(batch_err)?;
    Ok(Json(ApiResponse::success(json!({
        "auto_approved": result.auto_approved,
        "pending_approval": result.pending_approval,
        "total": result.total,
    }))))
}

/// POST /api/v1/erp/color-prices/:id/approve - 审批
pub async fn approve_color_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ApproveColorPriceDto>,
) -> Result<Json<ApiResponse<ColorPriceDetail>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let user_id = auth.user_id as i64;
    let service = ColorPriceBatchService::from_state(&state);

    let m = service
        .approve(id, tenant_id, user_id, dto)
        .await
        .map_err(batch_err)?;
    Ok(Json(ApiResponse::success(model_to_detail(m))))
}

// ----------------------------------------------------------------------
// 价格历史（1 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-prices/:id/history - 价格历史
pub async fn get_color_price_history(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<PagedResponse<PriceHistoryItem>>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceHistoryService::from_state(&state);

    let (items, total) = service.list_by_price(id, tenant_id, 1, 100).await
        .map_err(|e| AppError::database(e.to_string()))?;
    let page_items: Vec<PriceHistoryItem> = items
        .into_iter()
        .map(|m| PriceHistoryItem {
            id: m.id,
            product_color_price_id: m.product_color_price_id,
            old_price: m.old_price,
            new_price: m.new_price,
            currency: m.currency,
            change_type: m.change_type,
            change_reason: m.change_reason,
            change_percent: m.change_percent,
            quantity: m.quantity,
            operated_by: m.operated_by,
            operated_at: m.operated_at,
            approved_by: m.approved_by,
            approved_at: m.approved_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: page_items,
        total,
        page: 1,
        page_size: 100,
    })))
}

// ----------------------------------------------------------------------
// 价格计算（1 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-prices/calculate - 价格计算
pub async fn calculate_color_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(req): Query<PriceCalcQuery>,
) -> Result<Json<ApiResponse<crate::models::color_price_dto::PriceCalcResult>>, AppError> {
    let _tenant_id = extract_tenant_id(&auth)? as i64;
    let calc_req = PriceCalcRequest {
        product_id: req.product_id,
        color_id: req.color_id,
        customer_id: req.customer_id,
        customer_level: req.customer_level,
        quantity: Decimal::from_str(&req.quantity)
            .map_err(|_| AppError::validation("无效的数量".to_string()))?,
        season: req.season,
        product_category_id: req.product_category_id,
        currency: req.currency.unwrap_or_else(|| "CNY".to_string()),
        calc_date: req.calc_date,
    };
    let result = crate::utils::price_calculator::calculate_price(&state.db, &calc_req)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(ApiResponse::success(result)))
}

/// 价格计算查询参数
#[derive(Debug, serde::Deserialize)]
pub struct PriceCalcQuery {
    pub product_id: i64,
    pub color_id: i64,
    pub customer_id: Option<i64>,
    pub customer_level: Option<String>,
    pub quantity: String, // Decimal 不能直接 query，需 String
    pub season: Option<String>,
    pub product_category_id: Option<i64>,
    pub currency: Option<String>,
    pub calc_date: Option<chrono::NaiveDate>,
}

// ----------------------------------------------------------------------
// 阶梯价（3 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-prices/tiers/:price_id - 阶梯价列表
pub async fn list_tiers(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(price_id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceTierService::from_state(&state);

    let items = service
        .list_by_price(price_id, tenant_id)
        .await
        .map_err(tier_err)?;
    Ok(Json(ApiResponse::success(json!({ "items": items, "total": items.len() }))))
}

/// POST /api/v1/erp/color-prices/tiers - 新建阶梯价
pub async fn create_tier(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreatePriceTierDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceTierService::from_state(&state);

    let m = service.create(dto, tenant_id).await.map_err(tier_err)?;
    Ok(Json(ApiResponse::success(json!(m))))
}

/// DELETE /api/v1/erp/color-prices/tiers/item/:tier_id - 删除阶梯价
pub async fn delete_tier(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(tier_id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceTierService::from_state(&state);

    service.delete(tier_id, tenant_id).await.map_err(tier_err)?;
    Ok(Json(ApiResponse::success(json!({ "deleted": tier_id }))))
}

// ----------------------------------------------------------------------
// 客户专属价（2 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-prices/customer-special - 客户专属价列表
pub async fn list_customer_special_prices(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(_query): Query<ListCustomerColorPricesQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _tenant_id = extract_tenant_id(&auth)? as i64;
    use crate::models::customer_color_price;
    use sea_orm::EntityTrait;
    let items: Vec<customer_color_price::Model> = customer_color_price::Entity::find()
        .all(&*state.db)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(ApiResponse::success(json!({
        "items": items,
        "total": items.len(),
    }))))
}

/// POST /api/v1/erp/color-prices/customer-special - 新建客户专属价
pub async fn create_customer_special_price(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateCustomerColorPriceDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    use crate::models::customer_color_price;
    use sea_orm::{Set};

    let now = chrono::Utc::now();
    let active = customer_color_price::ActiveModel {
        id: Default::default(),
        customer_id: Set(dto.customer_id),
        product_id: Set(dto.product_id),
        color_id: Set(dto.color_id),
        special_price: Set(dto.special_price),
        discount_percent: Set(dto.discount_percent),
        currency: Set(dto.currency),
        valid_from: Set(dto.valid_from),
        valid_until: Set(dto.valid_until),
        notes: Set(dto.notes),
        approved_by: Set(None),
        approved_at: Set(None),
        tenant_id: Set(tenant_id),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let result = customer_color_price::Entity::insert(active)
        .exec_with_returning(&*state.db)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(ApiResponse::success(json!(result))))
}

// ----------------------------------------------------------------------
// 季节调价规则（2 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-prices/seasonal-rules - 季节规则列表
pub async fn list_seasonal_rules(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListSeasonalRulesQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceSeasonalService::from_state(&state);

    let (items, total) = service.list(tenant_id, &query).await.map_err(seasonal_err)?;
    Ok(Json(ApiResponse::success(json!({
        "items": items,
        "total": total,
        "page": query.page.unwrap_or(1),
        "page_size": query.page_size.unwrap_or(20),
    }))))
}

/// POST /api/v1/erp/color-prices/seasonal-rules - 新建季节规则
pub async fn create_seasonal_rule(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateSeasonalRuleDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceSeasonalService::from_state(&state);

    let m = service.create(dto, tenant_id).await.map_err(seasonal_err)?;
    Ok(Json(ApiResponse::success(json!(m))))
}

/// DELETE /api/v1/erp/color-prices/seasonal-rules/:id - 软删除季节规则
pub async fn delete_seasonal_rule(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorPriceSeasonalService::from_state(&state);

    service.delete(id, tenant_id).await.map_err(seasonal_err)?;
    Ok(Json(ApiResponse::success(json!({ "deleted": id }))))
}
