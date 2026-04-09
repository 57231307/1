use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DyeBatch {
    pub id: i32,
    pub batch_no: String,
    pub recipe_code: String,
    pub greige_code: String,
    pub total_weight_kg: f64,
    pub status: String,
    #[serde(default)]
    pub vat_capacity: f64,
    #[serde(default)]
    pub liquor_ratio: String,
}
