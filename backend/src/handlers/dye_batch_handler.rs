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
    pub color_code: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDyeBatchRequest {
    pub batch_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<f64>,
    pub status: Option<String>,
    pub production_date: Option<DateTime<Utc>>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDyeBatchRequest {
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<f64>,
    pub status: Option<String>,
    pub completion_date: Option<DateTime<Utc>>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteDyeBatchRequest {
    pub quality_grade: String,
    pub remarks: Option<String>,
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
    if let Some(color_code) = &query.color_code {
        q = q.filter(dye_batch::Column::ColorCode.contains(color_code));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }
    if let Some(grade) = &query.quality_grade {
        q = q.filter(dye_batch::Column::QualityGrade.eq(grade));
    }
    if let Some(start) = &query.start_date {
        q = q.filter(dye_batch::Column::ProductionDate.gte(*start));
    }
    if let Some(end) = &query.end_date {
        q = q.filter(dye_batch::Column::ProductionDate.lte(*end));
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
        color_code: Set(req.color_code),
        color_name: Set(req.color_name),
        fabric_type: Set(req.fabric_type),
        weight_kg: Set(req.weight_kg.and_then(Decimal::from_f64_retain)),
        status: Set(req.status.unwrap_or_else(|| "待生产".to_string())),
        production_date: Set(req.production_date),
        completion_date: Set(None),
        quality_grade: Set(req.quality_grade),
        remarks: Set(req.remarks),
        created_by: Set(req.created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        is_deleted: sea_orm::ActiveValue::NotSet,
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

    if let Some(color_code) = req.color_code {
        batch.color_code = Set(color_code);
    }
    if let Some(color_name) = req.color_name {
        batch.color_name = Set(color_name);
    }
    if let Some(fabric_type) = req.fabric_type {
        batch.fabric_type = Set(Some(fabric_type));
    }
    if let Some(weight_kg) = req.weight_kg {
        batch.weight_kg = Set(Decimal::from_f64_retain(weight_kg));
    }
    if let Some(status) = req.status {
        batch.status = Set(status);
    }
    if let Some(completion_date) = req.completion_date {
        batch.completion_date = Set(Some(completion_date));
    }
    if let Some(quality_grade) = req.quality_grade {
        batch.quality_grade = Set(Some(quality_grade));
    }
    if let Some(remarks) = req.remarks {
        batch.remarks = Set(Some(remarks));
    }

    batch.updated_at = Set(Utc::now());

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

pub async fn complete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<CompleteDyeBatchRequest>,
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

    batch.status = Set("已完成".to_string());
    batch.completion_date = Set(Some(Utc::now()));
    batch.quality_grade = Set(Some(req.quality_grade));
    if let Some(remarks) = req.remarks {
        batch.remarks = Set(Some(remarks));
    }
    batch.updated_at = Set(Utc::now());

    match batch.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "缸号完成成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("完成缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_dye_batches_by_color(
    State(state): State<AppState>,
    Path(color_code): Path<String>,
) -> impl IntoResponse {
    match dye_batch::Entity::find()
        .filter(dye_batch::Column::ColorCode.eq(color_code))
        .order_by_desc(dye_batch::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(batches) => (StatusCode::OK, Json(ApiResponse::success(batches))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取缸号列表失败：{}", e))),
        )
            .into_response(),
    }
}
