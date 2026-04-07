use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::product_category;
use crate::services::product_category_service::ProductCategoryService;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 产品类别列表
#[derive(Debug, Deserialize)]
pub struct ProductCategoryListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建产品类别请求
#[derive(Debug, Deserialize)]
pub struct CreateProductCategoryRequest {
    pub name: String,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
}

/// 更新产品类别请求
#[derive(Debug, Deserialize)]
pub struct UpdateProductCategoryRequest {
    pub name: Option<String>,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
}

/// 获取产品类别列表
pub async fn list_product_categories(
    State(state): State<AppState>,
    Query(query): Query<ProductCategoryListQuery>,
) -> Result<Json<ApiResponse<Vec<product_category::Model>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let category_service = ProductCategoryService::new(state.db.clone());
    let (categories, total) = category_service
        .list_categories(page, page_size, query.parent_id, query.search)
        .await?;

    Ok(Json(
        PaginatedResponse::new(categories, total, page, page_size).into(),
    ))
}

/// 获取产品类别详情
pub async fn get_product_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<product_category::Model>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    let category = category_service.get_category(id).await?;
    Ok(Json(ApiResponse::success(category)))
}

/// 创建产品类别
pub async fn create_product_category(
    State(state): State<AppState>,
    Json(req): Json<CreateProductCategoryRequest>,
) -> Result<Json<ApiResponse<product_category::Model>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    let category = category_service
        .create_category(req.name, req.parent_id, req.description)
        .await?;
    Ok(Json(ApiResponse::success_with_msg(
        category,
        "产品类别创建成功",
    )))
}

/// 更新产品类别
pub async fn update_product_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProductCategoryRequest>,
) -> Result<Json<ApiResponse<product_category::Model>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    let category = category_service
        .update_category(id, req.name, req.parent_id, req.description)
        .await?;
    Ok(Json(ApiResponse::success_with_msg(
        category,
        "产品类别更新成功",
    )))
}

/// 删除产品类别
pub async fn delete_product_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    category_service.delete_category(id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "产品类别删除成功")))
}

/// 获取产品类别树形结构
pub async fn get_product_category_tree(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<product_category::Model>>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    let tree = category_service.get_category_tree().await?;
    Ok(Json(ApiResponse::success(tree)))
}
