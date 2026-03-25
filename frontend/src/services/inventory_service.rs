use crate::services::api::ApiService;

/// 库存数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InventoryStock {
    pub id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub quantity: i32,
    pub unit: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 库存列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryStockListResponse {
    pub stock_list: Vec<InventoryStock>,
    pub total: u64,
}

/// 低库存预警数据
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LowStockAlert {
    pub id: i32,
    pub product_name: String,
    pub warehouse_name: String,
    pub current_quantity: i32,
    pub min_quantity: i32,
}

/// 库存服务
pub struct InventoryService;

impl InventoryService {
    pub async fn list_stock() -> Result<InventoryStockListResponse, String> {
        ApiService::get::<InventoryStockListResponse>("/api/v1/erp/inventory/stock").await
    }

    pub async fn get_stock(id: i32) -> Result<InventoryStock, String> {
        ApiService::get::<InventoryStock>(&format!("/api/v1/erp/inventory/stock/{}", id)).await
    }

    pub async fn create_stock(stock: InventoryStock) -> Result<InventoryStock, String> {
        let payload = serde_json::to_value(&stock).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/inventory/stock", &payload).await
    }

    /// 检查低库存
    pub async fn check_low_stock() -> Result<Vec<LowStockAlert>, String> {
        ApiService::get::<Vec<LowStockAlert>>("/api/v1/erp/inventory/stock/low-stock").await
    }
}

/// 库存调拨服务
pub struct InventoryTransferService;

/// 调拨单数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InventoryTransfer {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub from_warehouse_name: String,
    pub to_warehouse_id: i32,
    pub to_warehouse_name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 调拨单列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryTransferListResponse {
    pub transfers: Vec<InventoryTransfer>,
    pub total: u64,
}

impl InventoryTransferService {
    pub async fn list_transfers() -> Result<InventoryTransferListResponse, String> {
        ApiService::get::<InventoryTransferListResponse>("/api/v1/erp/inventory/transfers").await
    }

    pub async fn get_transfer(id: i32) -> Result<InventoryTransfer, String> {
        ApiService::get::<InventoryTransfer>(&format!("/api/v1/erp/inventory/transfers/{}", id)).await
    }

    pub async fn create_transfer(transfer: InventoryTransfer) -> Result<InventoryTransfer, String> {
        let payload = serde_json::to_value(&transfer).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/inventory/transfers", &payload).await
    }

    pub async fn approve_transfer(id: i32) -> Result<(), String> {
        ApiService::post(&format!("/api/v1/erp/inventory/transfers/{}/approve", id), &serde_json::json!({})).await
    }

    pub async fn delete_transfer(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/inventory/transfers/{}", id)).await
    }
}

/// 库存盘点服务
pub struct InventoryCountService;

/// 盘点单数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InventoryCount {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub status: String,
    pub count_date: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 盘点单列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryCountListResponse {
    pub counts: Vec<InventoryCount>,
    pub total: u64,
}

impl InventoryCountService {
    pub async fn list_counts() -> Result<InventoryCountListResponse, String> {
        ApiService::get::<InventoryCountListResponse>("/api/v1/erp/inventory/counts").await
    }

    pub async fn get_count(id: i32) -> Result<InventoryCount, String> {
        ApiService::get::<InventoryCount>(&format!("/api/v1/erp/inventory/counts/{}", id)).await
    }

    pub async fn create_count(count: InventoryCount) -> Result<InventoryCount, String> {
        let payload = serde_json::to_value(&count).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/inventory/counts", &payload).await
    }

    pub async fn approve_count(id: i32) -> Result<(), String> {
        ApiService::post(&format!("/api/v1/erp/inventory/counts/{}/approve", id), &serde_json::json!({})).await
    }

    pub async fn complete_count(id: i32) -> Result<(), String> {
        ApiService::post(&format!("/api/v1/erp/inventory/counts/{}/complete", id), &serde_json::json!({})).await
    }

    pub async fn delete_count(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/inventory/counts/{}", id)).await
    }
}
