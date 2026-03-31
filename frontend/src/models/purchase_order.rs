use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: String,
    pub expected_delivery_date: Option<String>,
    pub status: String,
    pub total_amount: String,
    pub paid_amount: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderItem {
    pub id: i32,
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: String,
    pub material_name: String,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: String,
    pub quantity_ordered: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt_ordered: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub delivery_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseOrderRequest {
    pub supplier_id: i32,
    pub order_date: String,
    pub expected_delivery_date: Option<String>,
    pub warehouse_id: i32,
    pub department_id: i32,
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderItemRequest {
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: String,
    pub material_name: String,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: String,
    pub currency: Option<String>,
    pub quantity_ordered: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt_ordered: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub delivery_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseOrderRequest {
    pub supplier_id: Option<i32>,
    pub order_date: Option<String>,
    pub expected_delivery_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateOrderItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: Option<String>,
    pub quantity_ordered: Option<String>,
    pub tax_rate: Option<String>,
    pub delivery_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RejectOrderRequest {
    pub reason: String,
}
