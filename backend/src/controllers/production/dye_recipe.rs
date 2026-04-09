use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct DyeRecipe {
    pub id: i32,
    pub recipe_code: String,
    pub color_name: String,
    pub fabric_type: String,
    pub dyes: String,
    pub temp_c: i32,
    pub time_mins: i32,
}

#[derive(Deserialize)]
pub struct CreateDyeRecipeDto {
    pub recipe_code: String,
    pub color_name: String,
    pub fabric_type: String,
    pub dyes: String,
    pub temp_c: i32,
    pub time_mins: i32,
}

pub async fn get_list() -> impl IntoResponse {
    let list = vec![
        DyeRecipe {
            id: 1,
            recipe_code: "RECIPE-001".to_string(),
            color_name: "大红".to_string(),
            fabric_type: "全棉".to_string(),
            dyes: "活性红: 2.5%, 元明粉: 40g/L".to_string(),
            temp_c: 60,
            time_mins: 45,
        },
    ];
    Json(list)
}

pub async fn create(Json(payload): Json<CreateDyeRecipeDto>) -> impl IntoResponse {
    let new_recipe = DyeRecipe {
        id: 2,
        recipe_code: payload.recipe_code,
        color_name: payload.color_name,
        fabric_type: payload.fabric_type,
        dyes: payload.dyes,
        temp_c: payload.temp_c,
        time_mins: payload.time_mins,
    };
    Json(new_recipe)
}
