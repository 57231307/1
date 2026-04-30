use crate::middleware::auth_context::AuthContext;
use crate::models::sales_price;
use crate::services::sales_price_service::{CreateSalesPriceInput, SalesPriceService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct SalesPriceQuery {
    pub product_id: Option<i32>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

pub async fn list_prices(
    Query(params): Query<SalesPriceQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_price::Model>>>, AppError> {
    info!("用户 {} 正在查询销售价格列表", auth.user_id);

    let service = SalesPriceService::new(state.db.clone());
    let query_params = crate::services::sales_price_service::SalesPriceQueryParams {
        product_id: params.product_id,
        customer_type: params.customer_type,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
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
    Json(_req): Json<ApprovePriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在批准销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    service.approve_price(id, auth.user_id).await?;
    info!("销售价格批准成功，ID: {}", id);

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

pub async fn list_strategies(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_price::Model>>>, AppError> {
    info!("用户 {} 正在查询销售价格策略", auth.user_id);

    let service = SalesPriceService::new(state.db.clone());
    let strategies = service.list_strategies().await?;
    info!("销售价格策略查询成功，共 {} 条记录", strategies.len());

    Ok(Json(ApiResponse::success(strategies)))
}
