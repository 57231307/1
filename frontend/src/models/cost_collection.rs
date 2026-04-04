use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCollection {
    pub id: i32,
    pub collection_no: String,
    pub collection_date: String,
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,
    pub direct_material: Value,
    pub direct_labor: Value,
    pub manufacturing_overhead: Value,
    pub processing_fee: Value,
    pub dyeing_fee: Value,
    pub output_quantity_meters: Option<Value>,
    pub output_quantity_kg: Option<Value>,
    pub created_by: i32,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CostCollectionQuery {
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCostCollectionRequest {
    pub collection_date: String,
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,
    pub direct_material: String,
    pub direct_labor: String,
    pub manufacturing_overhead: String,
    pub processing_fee: String,
    pub dyeing_fee: String,
    pub output_quantity_meters: Option<String>,
    pub output_quantity_kg: Option<String>,
}
