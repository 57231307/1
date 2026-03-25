use crate::services::api::ApiService;

/// 产品数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub category_id: Option<i32>,
    pub unit: String,
    pub price: Option<f64>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 产品列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProductListResponse {
    pub products: Vec<Product>,
    pub total: u64,
}

/// 创建产品请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub code: String,
    pub category_id: Option<i32>,
    pub unit: String,
    pub price: Option<f64>,
    pub description: Option<String>,
}

/// 更新产品请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Option<String>,
    pub price: Option<f64>,
    pub description: Option<String>,
}

/// 产品服务
/// 封装所有产品相关的 API 调用
pub struct ProductService;

impl ProductService {
    /// 获取产品列表
    pub async fn list_products() -> Result<ProductListResponse, String> {
        ApiService::get::<ProductListResponse>("/api/v1/erp/products").await
    }

    /// 获取产品详情
    pub async fn get_product(id: i32) -> Result<Product, String> {
        ApiService::get::<Product>(&format!("/api/v1/erp/products/{}", id)).await
    }

    /// 创建产品
    pub async fn create_product(req: CreateProductRequest) -> Result<Product, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/products", &payload).await
    }

    /// 更新产品
    pub async fn update_product(id: i32, req: UpdateProductRequest) -> Result<Product, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/api/v1/erp/products/{}", id), &payload).await
    }

    /// 删除产品
    pub async fn delete_product(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/products/{}", id)).await
    }
}
