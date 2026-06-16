//! 销售报价单 Handler 层
//!
//! Week 1 任务：仅实现 list_quotations / get_quotation / create_quotation。
//! 其他 13 个端点为占位实现，返回 NOT_IMPLEMENTED，
//! 等待 Week 2/3 任务（pricing/approval/convert）补全。
//! 创建时间: 2026-06-16

use crate::models::quotation_create_dto::CreateQuotationDto;
use crate::models::quotation_response_dto::{QuotationItemResponseDto, QuotationResponseDto};
use crate::middleware::auth_context::AuthContext;
use crate::services::quotation_service::{QuotationService, ServiceError};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

// ----------------------------------------------------------------------
// Task 5 实现的 3 个端点：list / get / create
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

/// GET /api/v1/erp/quotations
/// 列表查询
pub async fn list_quotations(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListQuotationsQuery>,
) -> Result<Json<ApiResponse<ListQuotationsResponse>>, AppError> {
    let service = QuotationService::from_state(&state);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

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
        .map(|model| {
            // 列表接口默认不加载 items/terms，减少响应体大小
            QuotationResponseDto::from(model)
        })
        .collect();

    Ok(Json(ApiResponse::success(ListQuotationsResponse {
        list: dtos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/quotations/:id
/// 详情查询（含明细与条款）
pub async fn get_quotation(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    let service = QuotationService::from_state(&state);
    let model = service.get_by_id(id).await?;

    // 加载明细
    use crate::models::sales_quotation_item;
    let items: Vec<QuotationItemResponseDto> = sales_quotation_item::Entity::find()
        .filter(sales_quotation_item::Column::QuotationId.eq(id))
        .all(&*state.db)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(Into::into)
        .collect();

    // 加载条款
    use crate::models::sales_quotation_term;
    let terms: Vec<crate::models::quotation_response_dto::QuotationTermResponseDto> =
        sales_quotation_term::Entity::find()
            .filter(sales_quotation_term::Column::QuotationId.eq(id))
            .all(&*state.db)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Into::into)
            .collect();

    let mut dto = QuotationResponseDto::from(model);
    dto.items = items;
    dto.terms = terms;

    Ok(Json(ApiResponse::success(dto)))
}

/// POST /api/v1/erp/quotations
/// 创建草稿
pub async fn create_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateQuotationDto>,
) -> Result<Json<ApiResponse<QuotationResponseDto>>, AppError> {
    // 输入验证
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

// ----------------------------------------------------------------------
// 占位端点（Week 2/3 任务接入）
// ----------------------------------------------------------------------

/// PUT /api/v1/erp/quotations/:id
pub async fn update_quotation(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
    Json(_dto): Json<serde_json::Value>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/:id/submit
pub async fn submit_quotation(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/:id/approve
pub async fn approve_quotation(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/:id/reject
pub async fn reject_quotation(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/:id/cancel
pub async fn cancel_quotation(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/:id/convert
pub async fn convert_to_sales_order(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET /api/v1/erp/quotations/:id/terms
pub async fn get_quotation_terms(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// PUT /api/v1/erp/quotations/:id/terms
pub async fn set_quotation_terms(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET /api/v1/erp/quotations/expiring
pub async fn list_expiring(
    _auth: AuthContext,
    State(_state): State<AppState>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET /api/v1/erp/quotations/expired
pub async fn list_expired(
    _auth: AuthContext,
    State(_state): State<AppState>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/calculate-price
pub async fn calculate_price(
    _auth: AuthContext,
    State(_state): State<AppState>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// GET /api/v1/erp/quotations/color-prices/:product_color_id
pub async fn list_color_prices(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_product_color_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /api/v1/erp/quotations/color-prices/:product_color_id
pub async fn set_color_price(
    _auth: AuthContext,
    State(_state): State<AppState>,
    Path(_product_color_id): Path<i64>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

// 让 ServiceError 可以转 AppError
impl From<ServiceError> for AppError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound => AppError::not_found("报价单不存在"),
            ServiceError::InvalidState => {
                AppError::validation("当前状态不允许此操作".to_string())
            }
            ServiceError::Validation(msg) => AppError::validation(msg),
            ServiceError::Database(db_err) => AppError::internal(db_err.to_string()),
        }
    }
}
