use crate::models::greige_fabric::GreigeFabric;
use crate::services::api::ApiService;

pub struct GreigeFabricService;

impl GreigeFabricService {
    pub async fn get_list() -> Result<Vec<GreigeFabric>, String> {
        ApiService::get("/production/greige").await
    }

    pub async fn create(fabric: &GreigeFabric) -> Result<GreigeFabric, String> {
        ApiService::post("/production/greige", fabric).await
    }
}
