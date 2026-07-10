//! 销售报价单 Handler 层
//!
//! Week 2 任务 9：实现全部 16 个端点。
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 9
//! 创建时间: 2026-06-16

use chrono::Utc;
// v9 P1-G 修复（修正）：仅移除未使用的 ActiveModelTrait（测试模块内有独立 import）。
// 保留 QueryFilter（主代码大量使用 .filter()）；保留 ColumnTrait（Column.eq 需要其支持）。
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::product_color_price;
// 批次 158 v11 真实接入：审批状态常量替代字符串字面量
use crate::models::status::approval;
use crate::models::quotation_create_dto::CreateQuotationDto;
use crate::models::quotation_response_dto::{
    QuotationItemResponseDto, QuotationResponseDto, QuotationTermResponseDto,
};
use crate::models::quotation_update_dto::UpdateQuotationDto;
use crate::services::quotation_approval_service::QuotationApprovalService;
use crate::services::quotation_convert_service::QuotationConvertService;
use crate::services::quotation_pricing_service::{PricingContext, QuotationPricingService};
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

use axum::{
    extract::{Path, Query, State},
    Json,
};

// ----------------------------------------------------------------------
// 公共 DTO
// ----------------------------------------------------------------------

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListQuotationsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i64>,
    pub sales_user_id: Option<i64>,
    pub keyword: Option<String>,
}

/// 列表响应
#[derive(Debug, Serialize)]
pub struct ListQuotationsResponse {
    pub list: Vec<QuotationResponseDto>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 拒绝请求体
#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

/// 即将到期 / 已过期查询参数
#[derive(Debug, Deserialize, Default)]
pub struct ExpiryQuery {
    pub days: Option<i32>,
}

/// 销售订单响应（简化）
#[derive(Debug, Serialize)]
pub struct SalesOrderResponse {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<crate::models::sales_order::Model> for SalesOrderResponse {
    fn from(m: crate::models::sales_order::Model) -> Self {
        Self {
            id: m.id,
            order_no: m.order_no,
            customer_id: m.customer_id,
            total_amount: m.total_amount,
            status: m.status,
            created_at: m.created_at,
        }
    }
}

// ----------------------------------------------------------------------
// CRUD：list / get / create / update
// ----------------------------------------------------------------------

/// GET /api/v1/erp/quotations
pub async fn list_quotations(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListQuotationsQuery>,
) -> Result<Json<ApiResponse<ListQuotationsResponse>>, AppError> {
    let service = QuotationService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let (items, total) = service
        .list(
            page,
            page_size,
            query.status,
            query.customer_id,
            query.sales_user_id,
            query.keyword,
        )
        .await?;

    let dtos: Vec<QuotationResponseDto> = items
        .into_iter()
        .map(QuotationResponseDto::from)
        .collect();

    Ok(Json(ApiResponse::success(ListQuotationsResponse {
        list: dtos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/quotations/:id
pub async fn get_quotation(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    let service = QuotationService::from_state(&state);
    let model = service.get_by_id(id).await?;

    let items: Vec<QuotationItemResponseDto> = crate::models::sales_quotation_item::Entity::find()
        .filter(crate::models::sales_quotation_item::Column::QuotationId.eq(id))
        .all(&*state.db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let terms: Vec<QuotationTermResponseDto> = crate::models::sales_quotation_term::Entity::find()
        .filter(crate::models::sales_quotation_term::Column::QuotationId.eq(id))
        .all(&*state.db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let mut dto = QuotationResponseDto::from(model);
    dto.items = items;
    dto.terms = terms;

    Ok(Json(ApiResponse::success(dto)))
}

/// POST /api/v1/erp/quotations
pub async fn create_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateQuotationDto>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    use validator::Validate;
    if let Err(e) = dto.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let service = QuotationService::from_state(&state);
    let model = service
        .create_draft(dto, auth.user_id as i64)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        QuotationResponseDto::from(model),
        "报价单草稿创建成功",
    )))
}

/// PUT /api/v1/erp/quotations/:id
// 批次 94 P2-13 修复：移除 let _ = auth; 占位，注入 auth.user_id 到 service.update 用于审计日志
pub async fn update_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateQuotationDto>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    use validator::Validate;
    if let Err(e) = dto.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let service = QuotationService::from_state(&state);
    let model = service.update(id, dto, auth.user_id as i64).await?;
    Ok(Json(ApiResponse::success_with_message(
        QuotationResponseDto::from(model),
        "报价单更新成功",
    )))
}

// ----------------------------------------------------------------------
// 审批流
// ----------------------------------------------------------------------

/// POST /api/v1/erp/quotations/:id/submit
pub async fn submit_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    let service = QuotationApprovalService::from_state(&state);
    let model = service.submit(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        QuotationResponseDto::from(model),
        "报价单已提交审批",
    )))
}

/// POST /api/v1/erp/quotations/:id/approve
pub async fn approve_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    let service = QuotationApprovalService::from_state(&state);
    let model = service.approve(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        QuotationResponseDto::from(model),
        "报价单已批准",
    )))
}

/// POST /api/v1/erp/quotations/:id/reject
pub async fn reject_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<RejectRequest>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    if body.reason.trim().is_empty() {
        return Err(AppError::validation("拒绝原因不能为空".to_string()));
    }
    let service = QuotationApprovalService::from_state(&state);
    let model = service.reject(id, auth.user_id, body.reason).await?;
    Ok(Json(ApiResponse::success_with_message(
        QuotationResponseDto::from(model),
        "报价单已拒绝",
    )))
}

/// POST /api/v1/erp/quotations/:id/cancel
pub async fn cancel_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    let service = QuotationService::from_state(&state);
    let model = service.cancel(id, auth.user_id as i64).await?;
    Ok(Json(ApiResponse::success_with_message(
        QuotationResponseDto::from(model),
        "报价单已取消",
    )))
}

// ----------------------------------------------------------------------
// 转换
// ----------------------------------------------------------------------

/// POST /api/v1/erp/quotations/:id/convert
pub async fn convert_to_sales_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<SalesOrderResponse>>, AppError> {
    let service = QuotationConvertService::from_state(&state);
    let order = service.convert(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        SalesOrderResponse::from(order),
        "报价单已转换为销售订单草稿",
    )))
}

// ----------------------------------------------------------------------
// 贸易条款
// ----------------------------------------------------------------------

/// GET /api/v1/erp/quotations/:id/terms
pub async fn get_quotation_terms(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<QuotationTermResponseDto>>>, AppError> {
    let terms: Vec<QuotationTermResponseDto> =
        crate::models::sales_quotation_term::Entity::find()
            .filter(crate::models::sales_quotation_term::Column::QuotationId.eq(id))
            .all(&*state.db)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
    Ok(Json(ApiResponse::success(terms)))
}

/// PUT /api/v1/erp/quotations/:id/terms
/// 全量替换报价单贸易条款
pub async fn set_quotation_terms(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(terms): Json<Vec<crate::models::quotation_create_dto::CreateQuotationTermDto>>,
) -> Result<Json<ApiResponse<Vec<QuotationTermResponseDto>>>, AppError> {
    use sea_orm::{ActiveModelTrait, Set, TransactionTrait};

    // 校验报价单存在
    // 批次 113 P1-8：移除 `let _ =` 显式丢弃，直接表达式语句校验存在性
    let service = QuotationService::from_state(&state);
    service.get_by_id(id).await?;

    let txn = state.db.begin().await?;

    // 删除旧条款
    crate::models::sales_quotation_term::Entity::delete_many()
        .filter(crate::models::sales_quotation_term::Column::QuotationId.eq(id))
        .exec(&txn)
        .await?;

    // 插入新条款
    for term in terms {
        let active = crate::models::sales_quotation_term::ActiveModel {
            id: Default::default(),
            quotation_id: Set(id),
            term_type: Set(term.term_type),
            term_key: Set(term.term_key),
            term_value: Set(term.term_value),
            sequence: Set(term.sequence),
        };
        active.insert(&txn).await?;
    }
    txn.commit().await?;

    // 重新查询返回
    let new_terms: Vec<QuotationTermResponseDto> =
        crate::models::sales_quotation_term::Entity::find()
            .filter(crate::models::sales_quotation_term::Column::QuotationId.eq(id))
            .all(&*state.db)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
    Ok(Json(ApiResponse::success_with_message(
        new_terms,
        "贸易条款已更新",
    )))
}

// ----------------------------------------------------------------------
// 状态分类：expiring / expired
// ----------------------------------------------------------------------

/// GET /api/v1/erp/quotations/expiring
/// 即将到期（默认 7 天内）
pub async fn list_expiring(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ExpiryQuery>,
) -> Result<Json<ApiResponse<Vec<QuotationResponseDto>>>, AppError> {
    let days = query.days.unwrap_or(7);
    let today = Utc::now().date_naive();
    let until = today + chrono::Duration::days(days as i64);

    use crate::models::sales_quotation;
    let items: Vec<QuotationResponseDto> = sales_quotation::Entity::find()
        .filter(sales_quotation::Column::Status.eq("approved"))
        .filter(sales_quotation::Column::ValidUntil.between(today, until))
        .all(&*state.db)
        .await?
        .into_iter()
        .map(QuotationResponseDto::from)
        .collect();
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/erp/quotations/expired
/// 已过期（valid_until < today 且状态非 cancelled/converted）
pub async fn list_expired(
    _auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<QuotationResponseDto>>>, AppError> {
    let today = Utc::now().date_naive();

    use crate::models::sales_quotation;
    use sea_orm::sea_query::Expr;
    let items: Vec<QuotationResponseDto> = sales_quotation::Entity::find()
        .filter(sales_quotation::Column::ValidUntil.lt(today))
        .filter(Expr::cust(
            "status NOT IN ('cancelled', 'converted', 'expired')",
        ))
        .all(&*state.db)
        .await?
        .into_iter()
        .map(QuotationResponseDto::from)
        .collect();
    Ok(Json(ApiResponse::success(items)))
}

// ----------------------------------------------------------------------
// 定价引擎
// ----------------------------------------------------------------------

/// POST /api/v1/erp/quotations/calculate-price
pub async fn calculate_price(
    _auth: AuthContext,
    State(state): State<AppState>,
    Json(ctx): Json<PricingContext>,
) -> Result<Json<ApiResponse<crate::services::quotation_pricing_service::PricingResult>>, AppError>
{
    let service = QuotationPricingService::from_state(&state);
    let result = service.calculate(ctx).await?;
    Ok(Json(ApiResponse::success(result)))
}

// ----------------------------------------------------------------------
// 色号价格
// ----------------------------------------------------------------------

/// 色号价格列表分页查询参数
#[derive(Debug, Deserialize)]
pub struct ColorPriceListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// GET /api/v1/erp/quotations/color-prices/:product_color_id
/// 列出该 product_color 下的所有价格档（分页查询，避免全表加载导致 OOM）
pub async fn list_color_prices(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(product_color_id): Path<i64>,
    Query(query): Query<ColorPriceListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<product_color_price::Model>>>, AppError> {
    // 页码采用 1-based 约定，page_size clamp 防止 DoS
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = product_color_price::Entity::find();
    // product_color_id 实际为 product_id + color_id 拼接：传 0 时不按产品过滤
    if product_color_id != 0 {
        q = q.filter(product_color_price::Column::ProductId.eq(product_color_id));
    }

    let paginator = q.paginate(&*state.db, page_size);
    let total = paginator.num_items().await?;
    // fetch_page 接收 0-based 页码，需将 1-based page 转换
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let items = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/quotations/color-prices/:product_color_id
/// 设置色号价格
pub async fn set_color_price(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(_product_color_id): Path<i64>,
    Json(payload): Json<ColorPriceUpsertRequest>,
) -> Result<Json<ApiResponse<product_color_price::Model>>, AppError> {
    use sea_orm::ActiveModelTrait;
    let mut active: product_color_price::ActiveModel = product_color_price::Model {
        id: payload.id.unwrap_or(0),
        product_id: payload.product_id,
        color_id: payload.color_id,
        currency: payload.currency.clone(),
        base_price: payload.base_price,
        effective_from: payload.effective_from,
        effective_to: payload.effective_to,
        customer_level: payload.customer_level.clone(),
        min_quantity: payload.min_quantity,
        notes: payload.notes.clone(),
        // P0-5 扩展字段（默认值以兼容旧 API）
        max_quantity: None,
        customer_id: None,
        season: None,
        is_active: true,
        priority: 0,
        created_by: None,
        approved_by: None,
        approved_at: None,
        approval_status: approval::APPROVED.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
    .into();

    if payload.id.is_some() {
        active.updated_at = Set(Utc::now());
        let updated = active.update(&*state.db).await?;
        Ok(Json(ApiResponse::success_with_message(
            updated,
            "色号价格已更新",
        )))
    } else {
        active.id = Default::default();
        active.created_at = Set(Utc::now());
        let inserted = active.insert(&*state.db).await?;
        Ok(Json(ApiResponse::success_with_message(
            inserted,
            "色号价格已创建",
        )))
    }
}

use sea_orm::Set;

/// 色号价格 upsert 请求
#[derive(Debug, Deserialize)]
pub struct ColorPriceUpsertRequest {
    pub id: Option<i64>,
    pub product_id: i64,
    pub color_id: i64,
    pub currency: String,
    pub base_price: rust_decimal::Decimal,
    pub effective_from: chrono::NaiveDate,
    pub effective_to: Option<chrono::NaiveDate>,
    pub customer_level: Option<String>,
    pub min_quantity: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
}

// ----------------------------------------------------------------------
// ServiceError → AppError
// ----------------------------------------------------------------------

impl From<ServiceError> for AppError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound => AppError::not_found("报价单不存在"),
            ServiceError::InvalidState => {
                AppError::validation("当前状态不允许此操作".to_string())
            }
            ServiceError::Validation(msg) => AppError::validation(msg),
            ServiceError::Database(db_err) => AppError::internal(db_err.to_string()),
            // 批次 265：paginate_with_total 返回的 AppError 直接透传
            ServiceError::App(e) => e,
        }
    }
}
