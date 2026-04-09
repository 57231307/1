use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCount {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: String,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountItem {
    pub id: i32,
    pub count_id: i32,
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub barcode: Option<String>,
    pub roll_length: Option<String>,
    pub dye_lot_no: Option<String>,
    pub quantity_before: String,
    pub quantity_actual: String,
    pub quantity_difference: String,
    pub unit_cost: String,
    pub total_cost: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountDetail {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: String,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<InventoryCountItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryCountQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub warehouse_id: Option<i32>,
    pub count_no: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryCountItemRequest {
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub quantity_actual: String,
    pub unit_cost: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryCountRequest {
    pub warehouse_id: i32,
    pub count_date: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub items: Option<Vec<CreateInventoryCountItemRequest>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInventoryCountRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<CreateInventoryCountItemRequest>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApproveCountRequest {
    pub approved: bool,
    pub notes: Option<String>,
}
