use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCategory {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductCategoryRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProductCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub is_enabled: Option<bool>,
}
