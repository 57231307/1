use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct GlobalUnitConstant {
    pub id: i32,
    pub from_unit: String,
    pub to_unit: String,
    pub ratio: f64,
}

#[derive(Serialize)]
pub struct ProductConversion {
    pub product_id: i32,
    pub product_code: String,
    pub product_name: String,
    pub width_cm: f64,
    pub weight_gsm: f64,
    pub meters_per_kg: f64,
}

pub async fn get_global_constants() -> impl IntoResponse {
    let constants = vec![
        GlobalUnitConstant { id: 1, from_unit: "码(yd)".to_string(), to_unit: "米(m)".to_string(), ratio: 0.9144 },
        GlobalUnitConstant { id: 2, from_unit: "磅(lb)".to_string(), to_unit: "公斤(kg)".to_string(), ratio: 0.453592 },
    ];
    Json(constants)
}

pub async fn get_product_conversions() -> impl IntoResponse {
    let products = vec![
        ProductConversion {
            product_id: 1,
            product_code: "FAB-001".to_string(),
            product_name: "全棉纯色汗布".to_string(),
            width_cm: 180.0,
            weight_gsm: 200.0,
            meters_per_kg: 2.7778, // 1000 / (1.8 * 200)
        },
        ProductConversion {
            product_id: 2,
            product_code: "FAB-002".to_string(),
            product_name: "涤纶网眼布".to_string(),
            width_cm: 160.0,
            weight_gsm: 130.0,
            meters_per_kg: 4.8077, // 1000 / (1.6 * 130)
        },
    ];
    Json(products)
}
