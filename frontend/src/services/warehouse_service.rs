use crate::models::warehouse::{
    CreateWarehouseRequest, Warehouse, WarehouseListResponse, UpdateWarehouseRequest,
};
use crate::services::api::ApiService;

pub struct WarehouseService;

impl WarehouseService {
    pub async fn list_warehouses() -> Result<WarehouseListResponse, String> {
        ApiService::get::<WarehouseListResponse>("/warehouses").await
    }

    pub async fn get_warehouse(id: i32) -> Result<Warehouse, String> {
        ApiService::get::<Warehouse>(&format!("/warehouses/{}", id)).await
    }

    pub async fn create_warehouse(req: CreateWarehouseRequest) -> Result<Warehouse, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/warehouses", &payload).await
    }

    pub async fn update_warehouse(id: i32, req: UpdateWarehouseRequest) -> Result<Warehouse, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/warehouses/{}", id), &payload).await
    }

    pub async fn delete_warehouse(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/warehouses/{}", id)).await
    }
}
