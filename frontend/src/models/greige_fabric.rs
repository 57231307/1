use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreigeFabric {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub width_cm: f64,
    pub weight_gsm: f64,
    pub composition: String,
    pub meters_per_kg: f64,
}
