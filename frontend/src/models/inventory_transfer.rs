use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransfer {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: String,
    pub status: String,
    pub total_quantity: String,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub shipped_at: Option<String>,
    pub received_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransferItem {
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub stock_id: Option<i32>,
    pub barcode: Option<String>,
    pub roll_length: Option<String>,
    pub dye_lot_no: Option<String>,
    pub quantity: String,
    pub shipped_quantity: String,
    pub received_quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransferDetail {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: String,
    pub status: String,
    pub total_quantity: String,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub shipped_at: Option<String>,
    pub received_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<InventoryTransferItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryTransferQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub from_warehouse_id: Option<i32>,
    pub to_warehouse_id: Option<i32>,
    pub transfer_no: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryTransferItemRequest {
    pub product_id: i32,
    pub quantity: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryTransferRequest {
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub items: Vec<CreateInventoryTransferItemRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInventoryTransferRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<CreateInventoryTransferItemRequest>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApproveTransferRequest {
    pub approved: bool,
    pub notes: Option<String>,
}
