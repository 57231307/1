use crate::models::unit_converter::{GlobalUnitConstant, ProductConversion};
use crate::services::api::ApiService;

pub struct UnitConverterService;

impl UnitConverterService {
    pub async fn get_global_constants() -> Result<Vec<GlobalUnitConstant>, String> {
        ApiService::get("/base/units/constants").await
    }

    pub async fn get_product_conversions() -> Result<Vec<ProductConversion>, String> {
        ApiService::get("/base/units/products").await
    }
}