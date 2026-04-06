//! 成本归集 Handler
//!
//! HTTP 接口层

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::middleware::auth_context::AuthContext;
use crate::models::cost_collection;
use crate::services::cost_collection_service::{
    CostCollectionService, CreateCostCollectionRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use rust_decimal::Decimal;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct CostCollectionQuery {
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建请求
#[derive(Debug, Deserialize)]

pub struct CreateCostCollectionRequestDto {
    pub collection_date: String,
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,
    pub direct_material: Decimal,
    pub direct_labor: Decimal,
    pub manufacturing_overhead: Decimal,
    pub processing_fee: Decimal,
    pub dyeing_fee: Decimal,
    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
}

/// 查询成本归集列表
pub async fn list_collections(
    Query(params): Query<CostCollectionQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<cost_collection::Model>>>, AppError> {
    info!("用户 {} 查询成本归集列表", auth.username);

    let service = CostCollectionService::new(state.db.clone());
    let (collections, total) = service
        .get_list(
            params.batch_no,
            params.color_no,
            params.page.unwrap_or(1),
            params.page_size.unwrap_or(20),
        )
        .await?;

    info!("用户 {} 查询成本归集成功，共 {} 条", auth.username, total);

    Ok(Json(ApiResponse::success(collections)))
}

/// 创建成本归集
#[axum::debug_handler]
pub async fn create_collection(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateCostCollectionRequestDto>,
) -> Result<Json<ApiResponse<cost_collection::Model>>, AppError> {
    info!(
        "用户 {} 创建成本归集，批次：{:?}",
        auth.username, req.batch_no
    );

    let collection_date = req.collection_date.parse().map_err(|e| {
        warn!("用户 {} 成本日期格式错误：{}", auth.username, e);
        AppError::ValidationError("成本日期格式错误".to_string())
    })?;

    let create_req = CreateCostCollectionRequest {
        collection_date,
        cost_object_type: req.cost_object_type,
        cost_object_id: req.cost_object_id,
        cost_object_no: req.cost_object_no,
        batch_no: req.batch_no,
        color_no: req.color_no,
        workshop: req.workshop,
        direct_material: req.direct_material,
        direct_labor: req.direct_labor,
        manufacturing_overhead: req.manufacturing_overhead,
        processing_fee: req.processing_fee,
        dyeing_fee: req.dyeing_fee,
        output_quantity_meters: req.output_quantity_meters,
        output_quantity_kg: req.output_quantity_kg,
    };

    let service = CostCollectionService::new(state.db.clone());
    let collection = service.create(create_req, auth.user_id).await?;
    info!(
        "用户 {} 创建成本归集成功：{}",
        auth.username, collection.collection_no
    );

    Ok(Json(ApiResponse::success_with_message(
        collection,
        "成本归集创建成功",
    )))
}
