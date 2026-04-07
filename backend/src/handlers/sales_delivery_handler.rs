use crate::middleware::auth_context::AuthContext;
use crate::models::dto::sales_delivery_dto::{CreateSalesDeliveryRequest, SalesDeliveryQueryParams};
use crate::services::sales_delivery_service::SalesDeliveryService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use validator::Validate;

pub async fn create_delivery(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesDeliveryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate().map_err(|e: validator::ValidationErrors| AppError::ValidationError(e.to_string()))?;

    let service = SalesDeliveryService::new(state.db.clone());
    let delivery = service.create_delivery(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(delivery)?,
        "销售交货单创建成功",
    )))
}

pub async fn list_deliveries(
    Query(params): Query<SalesDeliveryQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesDeliveryService::new(state.db.clone());
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let (deliveries, total) = service.list_deliveries(params).await?;

    let result = serde_json::json!({
        "items": deliveries,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}
