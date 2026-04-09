use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct GlobalUnitConstant {
    pub id: i32,
    pub from_unit: String,
    pub to_unit: String,
    pub ratio: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ProductConversion {
    pub product_id: i32,
    pub product_code: String,
    pub product_name: String,
    pub width_cm: f64,
    pub weight_gsm: f64,
    pub meters_per_kg: f64,
}