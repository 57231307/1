use crate::models::dye_batch::DyeBatch;
use crate::services::api::ApiService;

pub struct DyeBatchService;

impl DyeBatchService {
    pub async fn get_list() -> Result<Vec<DyeBatch>, String> {
        ApiService::get::<Vec<DyeBatch>>("/production/dye_batch").await
    }

    pub async fn create(req: &DyeBatch) -> Result<DyeBatch, String> {
        ApiService::post::<DyeBatch, DyeBatch>("/production/dye_batch", req).await
    }
}
