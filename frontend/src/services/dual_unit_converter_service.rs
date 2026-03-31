use crate::services::api::ApiService;

/// 单位换算请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConvertUnitRequest {
    /// 原始数值
    pub value: String,
    /// 原始单位："meters" 或 "kg"
    pub from_unit: String,
    /// 克重 (g/m²)
    pub gram_weight: String,
    /// 幅宽 (cm)
    pub width_cm: String,
}

/// 单位换算响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConvertUnitResponse {
    /// 换算后的数值
    pub converted_value: String,
    /// 目标单位
    pub to_unit: String,
    /// 换算公式说明
    pub formula: String,
    /// 换算率
    pub conversion_rate: String,
}

/// 验证双计量单位一致性请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidateDualUnitRequest {
    /// 米数
    pub quantity_meters: String,
    /// 公斤数
    pub quantity_kg: String,
    /// 克重 (g/m²)
    pub gram_weight: String,
    /// 幅宽 (cm)
    pub width_cm: String,
    /// 允许误差率（可选，默认 0.5%）
    pub tolerance: Option<String>,
}

/// 验证双计量单位一致性响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ValidateDualUnitResponse {
    /// 是否一致
    pub is_valid: bool,
    /// 计算出的公斤数
    pub calculated_kg: String,
    /// 差异值
    pub difference: String,
    /// 允许的差异值
    pub allowed_difference: String,
    /// 误差率
    pub error_rate: String,
}

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