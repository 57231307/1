use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DyeRecipe {
    pub id: i32,
    pub recipe_code: String,
    pub color_name: String,
    pub fabric_type: String,
    pub dyes: String,
    pub temp_c: i32,
    pub time_mins: i32,
    #[serde(default)]
    pub heating_rate: String,
    #[serde(default)]
    pub dye_ingredients: String,
}
