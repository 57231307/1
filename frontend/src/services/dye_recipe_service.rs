//! 染色配方管理服务

use crate::models::api_response::ApiResponse;
use crate::models::dye_recipe::{
    ApproveRecipeRequest, CreateDyeRecipeRequest, CreateVersionRequest, DyeRecipe,
    DyeRecipeListResponse, DyeRecipeQuery, UpdateDyeRecipeRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct DyeRecipeService;

impl CrudService for DyeRecipeService {
    type Model = DyeRecipe;
    type ListResponse = DyeRecipeListResponse;
    type CreateRequest = CreateDyeRecipeRequest;
    type UpdateRequest = UpdateDyeRecipeRequest;

    fn base_path() -> &'static str {
        "/dye-recipe"
    }
}


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

    pub async fn approve(id: i32, req: ApproveRecipeRequest) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::post(&format!("/dye-recipe/{}/approve", id), &req).await?;
        response.into_result()
    }

    pub async fn create_version(id: i32, req: CreateVersionRequest) -> Result<DyeRecipe, String> {
        let response: ApiResponse<DyeRecipe> = ApiService::post(&format!("/dye-recipe/{}/version", id), &req).await?;
        response.into_result()
    }
}
