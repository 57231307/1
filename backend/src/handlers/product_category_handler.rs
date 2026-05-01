use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use serde::Deserialize;
use validator::Validate;

use crate::models::product_category;
use crate::services::product_category_service::ProductCategoryService;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 产品类别列表
#[derive(Debug, Deserialize, Validate)]
pub struct ProductCategoryListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建产品类别请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductCategoryRequest {
    #[validate(length(min = 1, max = 100, message = "类别名称不能为空且最长100字符"))]
    pub name: String,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
}

/// 更新产品类别请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductCategoryRequest {
    #[validate(length(min = 1, max = 100, message = "类别名称不能为空且最长100字符"))]
    pub name: Option<String>,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
}

crate::define_crud_handlers!(
    ProductCategoryService,
    CreateProductCategoryRequest,
    UpdateProductCategoryRequest,
    ProductCategoryListQuery,
    i32
);

/// 获取产品类别树形结构
pub async fn get_product_category_tree(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<product_category::Model>>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    let tree = category_service.get_category_tree().await?;
    Ok(Json(ApiResponse::success(tree)))
}
