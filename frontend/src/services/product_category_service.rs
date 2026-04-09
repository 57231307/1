use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductCategory {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub parent_id: Option<i32>,
    pub level: i32,
    pub sort_order: i32,
    pub is_active: bool,
}

pub struct ProductCategoryService;

impl ProductCategoryService {
    pub async fn list() -> Result<Vec<ProductCategory>, String> {
        ApiService::get("/product-categories").await
    }
}
