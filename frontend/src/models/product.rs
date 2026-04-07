//! 产品模型

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub category_id: Option<i32>,
    pub unit: String,
    pub price: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProductListResponse {
    pub products: Vec<Product>,
    pub total: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub code: String,
    pub category_id: Option<i32>,
    pub unit: String,
    pub price: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Option<String>,
    pub price: Option<String>,
    pub description: Option<String>,
}
