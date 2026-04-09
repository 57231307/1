//! 产品模型

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub category_id: Option<i32>,
    pub specification: Option<String>,
    pub unit: String,
    pub standard_price: Option<f64>, // 大货价
    pub cost_price: Option<f64>,
    pub sample_price: Option<f64>,   // 剪样价/零剪价
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    
    // 面料行业字段
    #[serde(default)]
    pub product_type: String,
    #[serde(default)]
    pub fabric_composition: Option<String>,
    #[serde(default)]
    pub yarn_count: Option<String>,
    #[serde(default)]
    pub density: Option<String>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub gram_weight: Option<f64>,
    #[serde(default)]
    pub structure: Option<String>,
    #[serde(default)]
    pub finish: Option<String>,
    #[serde(default)]
    pub min_order_quantity: Option<f64>,
    #[serde(default)]
    pub lead_time: Option<i32>,
    
    #[serde(default)]
    pub stock_qty: Option<f64>,
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
