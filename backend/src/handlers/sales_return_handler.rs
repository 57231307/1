//! 销售退货 Handler
//!
//! 提供销售退货相关的 API 接口

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::sales_return_service::{
    CreateSalesReturnItemRequest, CreateSalesReturnRequest, SalesReturnService,
    UpdateSalesReturnRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 销售退货查询参数
#[derive(Deserialize)]
pub struct SalesReturnQueryParams {
    pub return_no: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 拒绝退货单请求
#[derive(Debug, Deserialize)]
pub struct RejectSalesReturnRequest {
    pub reason: String,
}

/// 路由注册
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_sales_returns).post(create_sales_return))
        .route(
            "/:id",
            get(get_sales_return)
                .put(update_sales_return)
                .delete(delete_sales_return),
        )
        .route("/:id/submit", post(submit_sales_return))
        .route("/:id/approve", post(approve_sales_return))
        .route("/:id/reject", post(reject_sales_return))
        .route("/:id/execute", post(execute_sales_return))
        .route(
            "/:id/items",
            get(list_return_items).post(create_return_item),
        )
        .route(
            "/:id/items/:item_id",
            put(update_return_item).delete(delete_return_item),
        )
}

/// 获取销售退货单列表
pub async fn list_sales_returns(
    State(state): State<AppState>,
    Query(params): Query<SalesReturnQueryParams>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<crate::models::sales_return::Model>>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    let (items, total) = service
        .list_returns(
            params.return_no,
            params.status,
            params.customer_id,
            page,
            page_size,
        )
        .await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// 获取销售退货单详情
pub async fn get_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.get_return(id).await?;
    Ok(Json(ApiResponse::success(return_order)))
}

/// 创建销售退货单
pub async fn create_sales_return(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesReturnRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.create_return(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 更新销售退货单
pub async fn update_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdateSalesReturnRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.update_return(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 删除销售退货单
pub async fn delete_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    service.delete_return(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "销售退货单已删除",
    )))
}

/// 提交销售退货单
pub async fn submit_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.submit_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 审批销售退货单
pub async fn approve_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.approve_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 拒绝销售退货单
pub async fn reject_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<RejectSalesReturnRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.reject_return(id, req.reason, auth.user_id).await?;
    Ok(Json(ApiResponse::success(return_order)))
}

/// 执行销售退货单
pub async fn execute_sales_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.execute_return(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(return_order)))
}

/// 获取退货单明细列表
pub async fn list_return_items(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crate::models::sales_return_item::Model>>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let items = service.list_return_items(id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// 添加退货明细项
pub async fn create_return_item(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<CreateSalesReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return_item::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let item = service.add_return_item(id, req).await?;
    Ok(Json(ApiResponse::success(item)))
}

/// 更新退货明细项
pub async fn update_return_item(
    State(state): State<AppState>,
    Path((_id, item_id)): Path<(i32, i32)>,
    _auth: AuthContext,
    Json(req): Json<UpdateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return_item::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let item = service
        .update_return_item(item_id, req.quantity, req.unit_price, req.reason)
        .await?;
    Ok(Json(ApiResponse::success(item)))
}

/// 删除退货明细项
pub async fn delete_return_item(
    State(state): State<AppState>,
    Path((_id, item_id)): Path<(i32, i32)>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    service.delete_return_item(item_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// 更新退货明细请求
#[derive(Debug, Deserialize)]
pub struct UpdateReturnItemRequest {
    pub quantity: Option<rust_decimal::Decimal>,
    pub unit_price: Option<rust_decimal::Decimal>,
    pub reason: Option<String>,
}
