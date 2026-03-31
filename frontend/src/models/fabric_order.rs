use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricOrder {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub order_date: String,
    pub required_date: String,
    pub status: String,
    pub total_amount: String,
    pub paid_amount: String,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricOrderItem {
    pub id: i32,
    pub fabric_order_id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub unit_price_meters: String,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub color_no: String,
    pub batch_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub base_price: Option<String>,
    pub color_extra_cost: Option<String>,
    pub grade_price_diff: Option<String>,
    pub final_price: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FabricOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
    pub status: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateFabricOrderRequest {
    pub customer_id: i32,
    pub order_date: String,
    pub required_date: String,
    pub items: Vec<FabricOrderItemRequest>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FabricOrderItemRequest {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub unit_price_meters: String,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub color_no: String,
    pub batch_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub base_price: Option<String>,
    pub color_extra_cost: Option<String>,
    pub grade_price_diff: Option<String>,
    pub final_price: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateFabricOrderRequest {
    pub required_date: Option<String>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<FabricOrderItemRequest>>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}
