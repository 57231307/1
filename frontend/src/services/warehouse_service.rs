use crate::services::api::ApiService;

/// 仓库数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Warehouse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 仓库列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WarehouseListResponse {
    pub warehouses: Vec<Warehouse>,
    pub total: u64,
}

/// 创建仓库请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateWarehouseRequest {
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
}

/// 更新仓库请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateWarehouseRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
}

/// 仓库服务
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
