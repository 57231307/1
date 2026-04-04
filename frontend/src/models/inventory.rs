use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub quantity_on_hand: String,
    pub quantity_available: String,
    pub quantity_reserved: String,
    pub reorder_point: String,
    pub bin_location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockListResponse {
    pub stock: Vec<StockResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockFabricResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_on_hand: String,
    pub quantity_available: String,
    pub quantity_reserved: String,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub bin_location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockFabricListResponse {
    pub stock: Vec<StockFabricResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockFabricRequest {
    pub warehouse_id: i32,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: String,
    pub quantity_kg: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub location_id: Option<i32>,
    pub shelf_no: Option<String>,
    pub layer_no: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: i32,
    pub transaction_type: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub quantity_before_meters: String,
    pub quantity_before_kg: String,
    pub quantity_after_meters: String,
    pub quantity_after_kg: String,
    pub source_bill_type: Option<String>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySummaryItem {
    pub product_id: i32,
    pub product_name: String,
    pub batch_no: String,
    pub color_no: String,
    pub grade: String,
    pub total_quantity_meters: String,
    pub total_quantity_kg: String,
    pub warehouse_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySummaryResponse {
    pub summary: Vec<InventorySummaryItem>,
    pub total_meters: String,
    pub total_kg: String,
}
