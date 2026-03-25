use crate::utils::error::AppError;
use crate::middleware::auth_context::AuthContext;
use crate::models::purchase_price;
use crate::services::purchase_price_service::{PurchasePriceService, CreatePurchasePriceInput};
use crate::utils::ApiResponse;
use axum::{
    extract::{Query, State, Path},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

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
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePriceRequest {
    pub price: String,
    pub expiry_date: Option<String>,
    pub status: Option<String>,
}

pub async fn list_prices(
    Query(params): Query<PurchasePriceQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<purchase_price::Model>>>, AppError> {
    info!("用户 {} 正在查询采购价格列表", auth.user_id);

    let service = PurchasePriceService::new(db);
    let query_params = crate::services::purchase_price_service::PurchasePriceQueryParams {
        product_id: params.product_id,
        supplier_id: params.supplier_id,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (prices, _total) = service.get_prices_list(query_params).await?;
    info!("采购价格列表查询成功，共 {} 条记录", prices.len());

    Ok(Json(ApiResponse::success(prices)))
}

pub async fn get_price(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<purchase_price::Model>>, AppError> {
    info!("用户 {} 正在查询采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(db);
    let price = service.get_price(id).await?;
    info!("采购价格查询成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn create_price(
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<CreatePurchasePriceInput>,
) -> Result<Json<ApiResponse<purchase_price::Model>>, AppError> {
    info!("用户 {} 正在创建采购价格，产品 ID: {}", auth.user_id, req.product_id);

    let service = PurchasePriceService::new(db);
    let price = service.create_price(req, auth.user_id).await?;
    info!("采购价格创建成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn update_price(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdatePriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在更新采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(db);
    service.update_price(
        id,
        req.price.parse().map_err(|e| AppError::ValidationError(format!("价格格式错误：{}", e)))?,
        req.expiry_date,
        req.status,
    ).await?;
    info!("采购价格更新成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn delete_price(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在删除采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(db);
    service.delete_price(id).await?;
    info!("采购价格删除成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn approve_price(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(_req): Json<ApprovePriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在批准采购价格，ID: {}", auth.user_id, id);

    let service = PurchasePriceService::new(db);
    service.approve_price(id, auth.user_id).await?;
    info!("采购价格批准成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

pub async fn get_price_history(
    State(db): State<Arc<DatabaseConnection>>,
    Path(material_id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<purchase_price::Model>>>, AppError> {
    info!("用户 {} 正在查询物料 {} 的价格历史", auth.user_id, material_id);

    let service = PurchasePriceService::new(db);
    let history = service.get_price_history(material_id).await?;
    info!("价格历史查询成功，共 {} 条记录", history.len());

    Ok(Json(ApiResponse::success(history)))
}
