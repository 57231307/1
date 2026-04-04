//! 基础数据配置处理器

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use std::sync::Arc;
use crate::services::base_data_service::{BaseDataService, BaseDataConfig, CreateBaseDataRequest, UpdateBaseDataRequest, ImportItem, ImportResult};
use crate::utils::response::ApiResponse;

#[derive(Debug, serde::Deserialize)]
pub struct CategoryQuery {
    pub category: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn list_base_data(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Json<ApiResponse<Vec<BaseDataConfig>>> {
    let service = BaseDataService::new(db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(50);

    match service.list(&query.category, page, page_size).await {
        Ok(items) => Json(ApiResponse::success(items, "获取成功")),
        Err(e) => Json(ApiResponse::error(&e.to_string())),
    }
}

pub async fn get_base_data(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Json<ApiResponse<BaseDataConfig>> {
    let service = BaseDataService::new(db);

    match service.get(id).await {
        Ok(item) => Json(ApiResponse::success(item, "获取成功")),
        Err(e) => Json(ApiResponse::error(&e.to_string())),
    }
}

pub async fn create_base_data(
    State(state): State<AppState>,
    Json(payload): Json<CreateBaseDataRequest>,
) -> Result<Json<ApiResponse<BaseDataConfig>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = BaseDataService::new(db);

    match service.create(payload).await {
        Ok(item) => Ok(Json(ApiResponse::success(item, "创建成功"))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(&e.to_string())),
        )),
    }
}

pub async fn update_base_data(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateBaseDataRequest>,
) -> Result<Json<ApiResponse<BaseDataConfig>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = BaseDataService::new(db);

    match service.update(id, payload).await {
        Ok(item) => Ok(Json(ApiResponse::success(item, "更新成功"))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(&e.to_string())),
        )),
    }
}

pub async fn delete_base_data(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Json<ApiResponse<()>> {
    let service = BaseDataService::new(db);

    match service.delete(id).await {
        Ok(_) => Json(ApiResponse::success((), "删除成功")),
        Err(e) => Json(ApiResponse::error(&e.to_string())),
    }
}

pub async fn import_base_data(
    State(state): State<AppState>,
    Path(category): Path<String>,
    Json(items): Json<Vec<ImportItem>>,
) -> Json<ApiResponse<ImportResult>> {
    let service = BaseDataService::new(db);

    match service.import(&category, items).await {
        Ok(result) => Json(ApiResponse::success(result, &format!(
            "导入完成：成功 {} 条，失败 {} 条",
            result.success_count, result.failed_count
        ))),
        Err(e) => Json(ApiResponse::error(&e.to_string())),
    }
}

pub async fn export_base_data(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Json<ApiResponse<Vec<BaseDataConfig>>> {
    let service = BaseDataService::new(db);

    match service.export_category(&category).await {
        Ok(items) => Json(ApiResponse::success(items, "导出成功")),
        Err(e) => Json(ApiResponse::error(&e.to_string())),
    }
}
