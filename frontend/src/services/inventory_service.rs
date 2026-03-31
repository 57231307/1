use crate::models::inventory::{
    StockResponse, StockListResponse, StockFabricResponse, StockFabricListResponse,
    CreateStockFabricRequest, TransactionListResponse, InventorySummaryResponse,
};
use crate::services::api::ApiService;

pub struct InventoryService;

impl InventoryService {
    pub async fn list_stock(page: u64, page_size: u64) -> Result<StockListResponse, String> {
        let url = format!("/inventory/stock?page={}&page_size={}", page, page_size);
        ApiService::get(&url).await
    }

    pub async fn get_stock(id: i32) -> Result<StockResponse, String> {
        ApiService::get(&format!("/inventory/stock/{}", id)).await
    }

    pub async fn create_stock(stock: &CreateStockFabricRequest) -> Result<StockFabricResponse, String> {
        let payload = serde_json::to_value(stock).map_err(|e| e.to_string())?;
        ApiService::post("/inventory/stock/fabric", &payload).await
    }

    pub async fn list_stock_fabric(
        page: u64,
        page_size: u64,
        batch_no: Option<&str>,
        color_no: Option<&str>,
    ) -> Result<StockFabricListResponse, String> {
        let mut url = format!("/inventory/stock/fabric?page={}&page_size={}", page, page_size);
        if let Some(batch) = batch_no {
            url.push_str(&format!("&batch_no={}", batch));
        }
        if let Some(color) = color_no {
            url.push_str(&format!("&color_no={}", color));
        }
        ApiService::get(&url).await
    }

    pub async fn list_transactions(
        page: u64,
        page_size: u64,
    ) -> Result<TransactionListResponse, String> {
        let url = format!("/inventory/stock/transactions?page={}&page_size={}", page, page_size);
        ApiService::get(&url).await
    }

    pub async fn get_inventory_summary() -> Result<InventorySummaryResponse, String> {
        ApiService::get("/inventory/stock/summary").await
    }
}

pub struct InventoryTransferService;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InventoryTransfer {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryTransferListResponse {
    pub transfers: Vec<InventoryTransfer>,
    pub total: u64,
}

impl InventoryTransferService {
    pub async fn list_transfers() -> Result<InventoryTransferListResponse, String> {
        ApiService::get("/inventory/transfers").await
    }

    pub async fn get_transfer(id: i32) -> Result<InventoryTransfer, String> {
        ApiService::get(&format!("/inventory/transfers/{}", id)).await
    }

    pub async fn create_transfer(transfer: &InventoryTransfer) -> Result<InventoryTransfer, String> {
        let payload = serde_json::to_value(transfer).map_err(|e| e.to_string())?;
        ApiService::post("/inventory/transfers", &payload).await
    }

    pub async fn approve_transfer(id: i32) -> Result<(), String> {
        ApiService::post(&format!("/inventory/transfers/{}/approve", id), &serde_json::json!({})).await
    }
}
