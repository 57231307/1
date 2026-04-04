use crate::models::product::{
    CreateProductRequest, Product, ProductListResponse, UpdateProductRequest,
};
use crate::services::api::ApiService;

pub struct ProductService;

impl ProductService {
    pub async fn list_products() -> Result<ProductListResponse, String> {
        ApiService::get::<ProductListResponse>("/products").await
    }

    pub async fn get_product(id: i32) -> Result<Product, String> {
        ApiService::get::<Product>(&format!("/products/{}", id)).await
    }

    pub async fn create_product(req: CreateProductRequest) -> Result<Product, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/products", &payload).await
    }

    pub async fn update_product(id: i32, req: UpdateProductRequest) -> Result<Product, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/products/{}", id), &payload).await
    }

    pub async fn delete_product(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/products/{}", id)).await
    }
}
