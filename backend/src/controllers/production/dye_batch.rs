use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct DyeBatch {
    pub id: i32,
    pub batch_no: String,
    pub recipe_code: String,
    pub greige_code: String,
    pub total_weight_kg: f64,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateDyeBatchDto {
    pub batch_no: String,
    pub recipe_code: String,
    pub greige_code: String,
    pub total_weight_kg: f64,
    pub status: String,
}

pub async fn get_list() -> impl IntoResponse {
    let list = vec![
        DyeBatch {
            id: 1,
            batch_no: "BATCH-20231001".to_string(),
            recipe_code: "RECIPE-001".to_string(),
            greige_code: "GF-001".to_string(),
            total_weight_kg: 350.5,
            status: "染色中".to_string(),
        },
    ];
    Json(list)
}

pub async fn create(Json(payload): Json<CreateDyeBatchDto>) -> impl IntoResponse {
    let new_batch = DyeBatch {
        id: 2,
        batch_no: payload.batch_no,
        recipe_code: payload.recipe_code,
        greige_code: payload.greige_code,
        total_weight_kg: payload.total_weight_kg,
        status: payload.status,
    };
    Json(new_batch)
}
