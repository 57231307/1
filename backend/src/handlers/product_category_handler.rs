use axum::{Json, extract::State};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::services::product_category_service::ProductCategoryService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 查询参数 - 产品类别列表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct ProductCategoryListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建产品类别请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductCategoryRequest {
    #[validate(length(min = 1, max = 100, message = "类别名称不能为空且最长100字符"))]
    pub name: String,
    #[validate(length(max = 50, message = "类别代码最长50字符"))]
    pub code: Option<String>,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
}

/// 更新产品类别请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductCategoryRequest {
    #[validate(length(min = 1, max = 100, message = "类别名称不能为空且最长100字符"))]
    pub name: Option<String>,
    #[validate(length(max = 50, message = "类别代码最长50字符"))]
    pub code: Option<String>,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
}

// define_crud_handlers! 出参实参 = service 的真实返回类型（批次 476 契约收紧）：
// list -> PaginatedResponse<product_category::Model>，get/create/update -> product_category::Model
// （证据 services/product_category_service.rs:24/61/69/104）
crate::define_crud_handlers!(
    ProductCategoryService,
    CreateProductCategoryRequest,
    UpdateProductCategoryRequest,
    ProductCategoryListQuery,
    i32,
    crate::utils::response::PaginatedResponse<crate::models::product_category::Model>,
    crate::models::product_category::Model
);

/// 获取产品类别树形结构
pub async fn get_product_category_tree(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let category_service = ProductCategoryService::new(state.db.clone());
    let tree = category_service.get_category_tree().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(tree)?)))
}
