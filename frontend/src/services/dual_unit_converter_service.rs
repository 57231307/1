use crate::models::dual_unit_converter::{
    ConvertUnitRequest, ConvertUnitResponse, ValidateDualUnitRequest, ValidateDualUnitResponse,
};
use crate::services::api::ApiService;

/// 双计量单位转换服务
pub struct DualUnitConverterService;

impl DualUnitConverterService {
    /// 米数转公斤数
    pub async fn meters_to_kg(
        value: &str,
        gram_weight: &str,
        width_cm: &str,
    ) -> Result<ConvertUnitResponse, String> {
        let req = ConvertUnitRequest {
            value: value.to_string(),
            from_unit: "meters".to_string(),
            gram_weight: gram_weight.to_string(),
            width_cm: width_cm.to_string(),
        };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/dual-unit-convert", &payload).await
    }

    /// 公斤数转米数
    pub async fn kg_to_meters(
        value: &str,
        gram_weight: &str,
        width_cm: &str,
    ) -> Result<ConvertUnitResponse, String> {
        let req = ConvertUnitRequest {
            value: value.to_string(),
            from_unit: "kg".to_string(),
            gram_weight: gram_weight.to_string(),
            width_cm: width_cm.to_string(),
        };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/dual-unit-convert", &payload).await
    }

    /// 验证双计量单位一致性
    pub async fn validate_dual_unit(
        quantity_meters: &str,
        quantity_kg: &str,
        gram_weight: &str,
        width_cm: &str,
        tolerance: Option<&str>,
    ) -> Result<ValidateDualUnitResponse, String> {
        let req = ValidateDualUnitRequest {
            quantity_meters: quantity_meters.to_string(),
            quantity_kg: quantity_kg.to_string(),
            gram_weight: gram_weight.to_string(),
            width_cm: width_cm.to_string(),
            tolerance: tolerance.map(|s| s.to_string()),
        };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/dual-unit-validate", &payload).await
    }
}