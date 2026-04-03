//! 坯布管理Handler（原料布匹管理）

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

use crate::models::greige_fabric;
use crate::utils::app_state::AppState;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[derive(Debug, Deserialize)]
pub struct GreigeFabricListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub fabric_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGreigeFabricRequest {
    pub fabric_no: String,
    pub fabric_name: String,
    pub fabric_type: String,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<DateTime<Utc>>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGreigeFabricRequest {
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockInRequest {
    pub warehouse_id: i32,
    pub location: Option<String>,
    pub weight_kg: f64,
    pub length_m: f64,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockOutRequest {
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub remarks: Option<String>,
}

pub async fn list_greige_fabrics(
    State(state): State<AppState>,
    Query(query): Query<GreigeFabricListQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let mut q = greige_fabric::Entity::find();

    if let Some(fabric_no) = &query.fabric_no {
        q = q.filter(greige_fabric::Column::FabricNo.contains(fabric_no));
    }
    if let Some(fabric_name) = &query.fabric_name {
        q = q.filter(greige_fabric::Column::FabricName.contains(fabric_name));
    }
    if let Some(fabric_type) = &query.fabric_type {
        q = q.filter(greige_fabric::Column::FabricType.eq(fabric_type));
    }
    if let Some(supplier_id) = query.supplier_id {
        q = q.filter(greige_fabric::Column::SupplierId.eq(supplier_id));
    }
    if let Some(warehouse_id) = query.warehouse_id {
        q = q.filter(greige_fabric::Column::WarehouseId.eq(warehouse_id));
    }
    if let Some(status) = &query.status {
        q = q.filter(greige_fabric::Column::Status.eq(status));
    }
    if let Some(grade) = &query.quality_grade {
        q = q.filter(greige_fabric::Column::QualityGrade.eq(grade));
    }

    q = q.order_by_desc(greige_fabric::Column::CreatedAt);

    match q.paginate(&*state.db, page_size).fetch_page(page - 1).await {
        Ok(fabrics) => {
            let total = fabrics.len() as u64;
            let paginated = PaginatedResponse::new(fabrics, total, page, page_size);
            paginated.into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取坯布列表失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_greige_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match greige_fabric::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(fabric)) => (StatusCode::OK, Json(ApiResponse::success(fabric))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("坯布不存在")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取坯布失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn create_greige_fabric(
    State(state): State<AppState>,
    Json(req): Json<CreateGreigeFabricRequest>,
) -> impl IntoResponse {
    let fabric = greige_fabric::ActiveModel {
        id: Set(0),
        fabric_no: Set(req.fabric_no),
        fabric_name: Set(req.fabric_name),
        fabric_type: Set(req.fabric_type),
        color_code: Set(req.color_code),
        width_cm: Set(req.width_cm.and_then(Decimal::from_f64_retain)),
        weight_kg: Set(req.weight_kg.and_then(Decimal::from_f64_retain)),
        length_m: Set(req.length_m.and_then(Decimal::from_f64_retain)),
        supplier_id: Set(req.supplier_id),
        batch_no: Set(req.batch_no),
        warehouse_id: Set(req.warehouse_id),
        location: Set(req.location),
        status: Set(req.status.unwrap_or_else(|| "在库".to_string())),
        quality_grade: Set(req.quality_grade),
        purchase_date: Set(req.purchase_date),
        remarks: Set(req.remarks),
        created_by: Set(req.created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    match fabric.insert(&*state.db).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_msg(created, "坯布创建成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("创建坯布失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn update_greige_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateGreigeFabricRequest>,
) -> impl IntoResponse {
    let mut fabric: greige_fabric::ActiveModel =
        match greige_fabric::Entity::find_by_id(id).one(&*state.db).await {
            Ok(Some(f)) => f.into(),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("坯布不存在")),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("获取坯布失败：{}", e))),
                )
                    .into_response();
            }
        };

    if let Some(fabric_name) = req.fabric_name {
        fabric.fabric_name = Set(fabric_name);
    }
    if let Some(fabric_type) = req.fabric_type {
        fabric.fabric_type = Set(fabric_type);
    }
    if let Some(color_code) = req.color_code {
        fabric.color_code = Set(Some(color_code));
    }
    if let Some(width_cm) = req.width_cm {
        fabric.width_cm = Set(Decimal::from_f64_retain(width_cm));
    }
    if let Some(weight_kg) = req.weight_kg {
        fabric.weight_kg = Set(Decimal::from_f64_retain(weight_kg));
    }
    if let Some(length_m) = req.length_m {
        fabric.length_m = Set(Decimal::from_f64_retain(length_m));
    }
    if let Some(supplier_id) = req.supplier_id {
        fabric.supplier_id = Set(Some(supplier_id));
    }
    if let Some(batch_no) = req.batch_no {
        fabric.batch_no = Set(Some(batch_no));
    }
    if let Some(warehouse_id) = req.warehouse_id {
        fabric.warehouse_id = Set(Some(warehouse_id));
    }
    if let Some(location) = req.location {
        fabric.location = Set(Some(location));
    }
    if let Some(status) = req.status {
        fabric.status = Set(status);
    }
    if let Some(quality_grade) = req.quality_grade {
        fabric.quality_grade = Set(Some(quality_grade));
    }
    if let Some(remarks) = req.remarks {
        fabric.remarks = Set(Some(remarks));
    }

    fabric.updated_at = Set(Utc::now());

    match fabric.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "坯布更新成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("更新坯布失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn delete_greige_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match greige_fabric::Entity::delete_by_id(id).exec(&*state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg((), "坯布删除成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("删除坯布失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn stock_in(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<StockInRequest>,
) -> impl IntoResponse {
    let mut fabric: greige_fabric::ActiveModel =
        match greige_fabric::Entity::find_by_id(id).one(&*state.db).await {
            Ok(Some(f)) => f.into(),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("坯布不存在")),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("获取坯布失败：{}", e))),
                )
                    .into_response();
            }
        };

    fabric.warehouse_id = Set(Some(req.warehouse_id));
    fabric.location = Set(req.location);
    fabric.weight_kg = Set(Decimal::from_f64_retain(req.weight_kg));
    fabric.length_m = Set(Decimal::from_f64_retain(req.length_m));
    fabric.status = Set("在库".to_string());
    if let Some(grade) = req.quality_grade {
        fabric.quality_grade = Set(Some(grade));
    }
    if let Some(remarks) = req.remarks {
        fabric.remarks = Set(Some(remarks));
    }
    fabric.updated_at = Set(Utc::now());

    match fabric.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "坯布入库成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("坯布入库失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn stock_out(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<StockOutRequest>,
) -> impl IntoResponse {
    let fabric = match greige_fabric::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("坯布不存在")),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("获取坯布失败：{}", e))),
            )
                .into_response();
        }
    };

    let mut update_fabric: greige_fabric::ActiveModel = fabric.clone().into();

    if let Some(out_weight) = req.weight_kg {
        let current_weight = fabric.weight_kg.unwrap_or(Decimal::ZERO);
        let new_weight = current_weight - Decimal::from_f64_retain(out_weight).unwrap_or(Decimal::ZERO);
        if new_weight < Decimal::ZERO {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("出库重量不能大于现有重量")),
            )
                .into_response();
        }
        update_fabric.weight_kg = Set(Some(new_weight));
    }

    if let Some(out_length) = req.length_m {
        let current_length = fabric.length_m.unwrap_or(Decimal::ZERO);
        let new_length = current_length - Decimal::from_f64_retain(out_length).unwrap_or(Decimal::ZERO);
        if new_length < Decimal::ZERO {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("出库长度不能大于现有长度")),
            )
                .into_response();
        }
        update_fabric.length_m = Set(Some(new_length));
    }

    update_fabric.status = Set("已出库".to_string());
    update_fabric.updated_at = Set(Utc::now());

    match update_fabric.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "坯布出库成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("坯布出库失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_greige_by_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<i32>,
) -> impl IntoResponse {
    match greige_fabric::Entity::find()
        .filter(greige_fabric::Column::SupplierId.eq(supplier_id))
        .order_by_desc(greige_fabric::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(fabrics) => (StatusCode::OK, Json(ApiResponse::success(fabrics))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取坯布列表失败：{}", e))),
        )
            .into_response(),
    }
}
