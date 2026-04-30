use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub id: i32,
    pub batch_no: String,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub color_no: String,
    pub color_name: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub stock_status: String,
    pub quality_status: String,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
    pub supplier_id: Option<i32>,
    pub supplier_name: Option<String>,
    pub purchase_order_no: Option<String>,
    pub remarks: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
    pub warehouse_id: Option<i32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateBatchRequest {
    pub batch_no: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub color_no: String,
    pub color_name: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
    pub supplier_id: Option<i32>,
    pub purchase_order_no: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBatchRequest {
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub expiry_date: Option<String>,
    pub remarks: Option<String>,
    pub stock_status: Option<String>,
    pub quality_status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferBatchRequest {
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub remarks: Option<String>,
}
