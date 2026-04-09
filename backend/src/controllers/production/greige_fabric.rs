use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct GreigeFabric {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub width_cm: f64,
    pub weight_gsm: f64,
    pub composition: String,
    pub meters_per_kg: f64,
}

#[derive(Deserialize)]
pub struct CreateGreigeFabricDto {
    pub code: String,
    pub name: String,
    pub width_cm: f64,
    pub weight_gsm: f64,
    pub composition: String,
}

pub async fn get_list() -> impl IntoResponse {
    let list = vec![
        GreigeFabric {
            id: 1,
            code: "GF-001".to_string(),
            name: "全棉坯布".to_string(),
            width_cm: 180.0,
            weight_gsm: 200.0,
            composition: "100% 棉".to_string(),
            meters_per_kg: 2.78,
        },
        GreigeFabric {
            id: 2,
            code: "GF-002".to_string(),
            name: "涤纶坯布".to_string(),
            width_cm: 160.0,
            weight_gsm: 130.0,
            composition: "100% 涤纶".to_string(),
            meters_per_kg: 4.81,
        },
    ];
    Json(list)
}

pub async fn create(Json(payload): Json<CreateGreigeFabricDto>) -> impl IntoResponse {
    let meters_per_kg = if payload.width_cm > 0.0 && payload.weight_gsm > 0.0 {
        1000.0 / (payload.width_cm / 100.0 * payload.weight_gsm)
    } else {
        0.0
    };

    let new_fabric = GreigeFabric {
        id: 3,
        code: payload.code,
        name: payload.name,
        width_cm: payload.width_cm,
        weight_gsm: payload.weight_gsm,
        composition: payload.composition,
        meters_per_kg,
    };
    Json(new_fabric)
}
