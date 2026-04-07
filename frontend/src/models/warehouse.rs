//! 仓库模型

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Warehouse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub warehouse_type: Option<String>,
    pub temperature_control: Option<bool>,
    pub humidity_control: Option<bool>,
    pub fire_protection_level: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WarehouseListResponse {
    pub warehouses: Vec<Warehouse>,
    pub total: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateWarehouseRequest {
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub warehouse_type: Option<String>,
    pub temperature_control: Option<bool>,
    pub humidity_control: Option<bool>,
    pub fire_protection_level: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateWarehouseRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub status: Option<String>,
    pub warehouse_type: Option<String>,
    pub temperature_control: Option<bool>,
    pub humidity_control: Option<bool>,
    pub fire_protection_level: Option<String>,
}
