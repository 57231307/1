//! 库存调整模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InventoryAdjustment {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub total_quantity: String,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: String,
    pub items: Vec<AdjustmentItem>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AdjustmentItem {
    pub id: i32,
    pub stock_id: i32,
    pub quantity: String,
    pub quantity_before: String,
    pub quantity_after: String,
    pub unit_cost: Option<String>,
    pub amount: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryAdjustmentListResponse {
    pub adjustments: Vec<AdjustmentSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdjustmentSummary {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_type: String,
    pub reason_type: String,
    pub status: String,
    pub total_quantity: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAdjustmentRequest {
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateAdjustmentItemRequest>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAdjustmentItemRequest {
    pub stock_id: i32,
    pub quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
}
