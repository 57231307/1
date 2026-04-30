//! 染色配方管理服务

use crate::models::api_response::ApiResponse;
use crate::models::dye_recipe::{
    ApproveRecipeRequest, CreateDyeRecipeRequest, CreateVersionRequest, DyeRecipe,
    DyeRecipeListResponse, DyeRecipeQuery, UpdateDyeRecipeRequest,
};
use crate::services::api::ApiService;

pub struct DyeRecipeService;

impl DyeRecipeService {
    pub async fn list(query: DyeRecipeQuery) -> Result<DyeRecipeListResponse, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(recipe_no) = &query.recipe_no {
            params.push(format!("recipe_no={}", recipe_no));
        }
        if let Some(color_code) = &query.color_code {
            params.push(format!("color_code={}", color_code));
        }
        if let Some(color_name) = &query.color_name {
            params.push(format!("color_name={}", color_name));
        }
        if let Some(dye_type) = &query.dye_type {
            params.push(format!("dye_type={}", dye_type));
        }
        if let Some(status) = &query.status {
            params.push(format!("status={}", status));
        }

        let url = if params.is_empty() {
            String::from("/dye-recipe")
        } else {
            format!("/dye-recipe?{}", params.join("&"))
        };
        let response: ApiResponse<DyeRecipeListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    pub async fn get(id: i32) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::get(&format!("/dye-recipe/{}", id)).await?;
        response.into_result()
    }

    pub async fn create(req: CreateDyeRecipeRequest) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::post("/dye-recipe", &req).await?;
        response.into_result()
    }

    pub async fn update(id: i32, req: UpdateDyeRecipeRequest) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::put(&format!("/dye-recipe/{}", id), &req).await?;
        response.into_result()
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/dye-recipe/{}", id)).await
    }

    pub async fn approve(id: i32, req: ApproveRecipeRequest) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::post(&format!("/dye-recipe/{}/approve", id), &req).await?;
        response.into_result()
    }

    pub async fn create_version(id: i32, req: CreateVersionRequest) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::post(&format!("/dye-recipe/{}/version", id), &req).await?;
        response.into_result()
    }
}
