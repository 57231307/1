//! 销售退货 Handler
//!
//! 提供销售退货相关的 API 接口

use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::sales_return_service::{
    CreateSalesReturnRequest, SalesReturnService, UpdateSalesReturnRequest,
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

/// 路由注册
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_returns).post(create_return))
        .route("/:id", put(update_return))
        .route("/:id/submit", post(submit_return))
        .route("/:id/approve", post(approve_return))
}

/// 获取销售退货单列表
pub async fn list_returns(
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

    Ok(Json(ApiResponse::success(PaginatedResponse {
        data: items.clone(),
        items,
        total,
        page,
        page_size,
    })))
}

/// 创建销售退货单
pub async fn create_return(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesReturnRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.create_return(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 更新销售退货单
pub async fn update_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdateSalesReturnRequest>,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.update_return(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 提交销售退货单
pub async fn submit_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.submit_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}

/// 审批销售退货单
pub async fn approve_return(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::models::sales_return::Model>>, AppError> {
    let service = SalesReturnService::new(state.db.clone());
    let return_order = service.approve_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(return_order)))
}
