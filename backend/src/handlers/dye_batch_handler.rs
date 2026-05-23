//! 缸号管理Handler（染色批次管理）

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;

use crate::models::dye_batch;
use crate::utils::app_state::AppState;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[derive(Debug, Deserialize)]
pub struct DyeBatchListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDyeBatchRequest {
    pub batch_no: String,
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDyeBatchRequest {
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
}

pub async fn list_dye_batches(
    State(state): State<AppState>,
    Query(query): Query<DyeBatchListQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let mut q = dye_batch::Entity::find();

    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    match q.paginate(&*state.db, page_size).fetch_page(page - 1).await {
        Ok(batches) => {
            let total = batches.len() as u64;
            let paginated = PaginatedResponse::new(batches, total, page, page_size);
            paginated.into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取缸号列表失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match dye_batch::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(batch)) => (StatusCode::OK, Json(ApiResponse::success(batch))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("缸号不存在")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn create_dye_batch(
    State(state): State<AppState>,
    Json(req): Json<CreateDyeBatchRequest>,
) -> impl IntoResponse {
    let batch = dye_batch::ActiveModel {
        id: Set(0),
        batch_no: Set(req.batch_no),
        greige_fabric_id: Set(req.greige_fabric_id),
        color_no: Set(req.color_no),
        planned_quantity: Set(req.planned_quantity.and_then(Decimal::from_f64_retain)),
        status: Set(req.status.or(Some("待生产".to_string()))),
        started_at: Set(None),
        completed_at: Set(None),
        is_deleted: Set(Some(false)),
        created_at: Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
        updated_at: Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
    };

    match batch.insert(&*state.db).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_msg(created, "缸号创建成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("创建缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn update_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateDyeBatchRequest>,
) -> impl IntoResponse {
    let mut batch: dye_batch::ActiveModel = match dye_batch::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(b)) => b.into(),
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("缸号不存在")),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("获取缸号失败：{}", e))),
            )
                .into_response();
        }
    };

    if let Some(greige_fabric_id) = req.greige_fabric_id {
        batch.greige_fabric_id = Set(Some(greige_fabric_id));
    }
    if let Some(color_no) = req.color_no {
        batch.color_no = Set(Some(color_no));
    }
    if let Some(planned_quantity) = req.planned_quantity {
        batch.planned_quantity = Set(Decimal::from_f64_retain(planned_quantity));
    }
    if let Some(status) = req.status {
        batch.status = Set(Some(status));
    }

    batch.updated_at = Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

    match batch.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "缸号更新成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("更新缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn delete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match dye_batch::Entity::delete_by_id(id).exec(&*state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg((), "缸号删除成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("删除缸号失败：{}", e))),
        )
            .into_response(),
    }
}
