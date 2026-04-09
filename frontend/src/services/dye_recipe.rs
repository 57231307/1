use crate::models::dye_recipe::DyeRecipe;
use crate::services::api::ApiService;

pub struct DyeRecipeService;

impl DyeRecipeService {
    pub async fn get_list() -> Result<Vec<DyeRecipe>, String> {
        ApiService::get::<Vec<DyeRecipe>>("/production/dye_recipe").await
    }

    pub async fn create(req: &DyeRecipe) -> Result<DyeRecipe, String> {
        ApiService::post::<DyeRecipe, DyeRecipe>("/production/dye_recipe", req).await
    }
}
