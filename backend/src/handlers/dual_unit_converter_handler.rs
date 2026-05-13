use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::ApiResponse;

/// 单位换算请求
#[derive(Debug, Deserialize)]
pub struct ConvertUnitRequest {
    /// 原始数值
    pub value: rust_decimal::Decimal,
    /// 原始单位："meters" 或 "kg"
    pub from_unit: String,
    /// 克重 (g/m²)
    pub gram_weight: rust_decimal::Decimal,
    /// 幅宽 (cm)
    pub width_cm: rust_decimal::Decimal,
}

/// 单位换算响应
#[derive(Debug, Serialize)]
pub struct ConvertUnitResponse {
    /// 换算后的数值
    pub converted_value: rust_decimal::Decimal,
    /// 目标单位
    pub to_unit: String,
    /// 换算公式说明
    pub formula: String,
    /// 换算率
    pub conversion_rate: rust_decimal::Decimal,
}

/// 双计量单位换算接口
///
/// # 请求示例
/// ```json
/// {
///     "value": "100.000",
///     "from_unit": "meters",
///     "gram_weight": "180.00",
///     "width_cm": "180.00"
/// }
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "success": true,
///     "data": {
///         "converted_value": "3.240",
///         "to_unit": "kg",
///         "formula": "公斤数 = 米数 × 克重 (g/m²) × 幅宽 (m) ÷ 1000",
///         "conversion_rate": "0.032400"
///     },
///     "message": "单位换算成功"
/// }
/// ```
pub async fn convert_dual_unit(Json(req): Json<ConvertUnitRequest>) -> impl IntoResponse {
    // 验证单位参数
    let from_unit = req.from_unit.to_lowercase();
    if from_unit != "meters" && from_unit != "kg" {
        return ApiResponse::<()>::error("无效的单位，必须是 'meters' 或 'kg'".to_string())
            .into_response();
    }

    // 执行换算
    let result = match from_unit.as_str() {
        "meters" => {
            // 米数转公斤数
            match DualUnitConverter::meters_to_kg(req.value, req.gram_weight, req.width_cm) {
                Ok(kg) => ConvertUnitResponse {
                    converted_value: kg,
                    to_unit: "kg".to_string(),
                    formula: format!(
                        "公斤数 = 米数 × 克重 (g/m²) × 幅宽 (m) ÷ 1000\n= {} × {} × {} ÷ 1000 = {}",
                        req.value,
                        req.gram_weight,
                        req.width_cm / rust_decimal::Decimal::from(100),
                        kg
                    ),
                    conversion_rate: DualUnitConverter::calculate_conversion_rate(
                        req.gram_weight,
                        req.width_cm,
                    )
                    .unwrap_or(rust_decimal::Decimal::ZERO),
                },
                Err(e) => return ApiResponse::<()>::error(e).into_response(),
            }
        }
        "kg" => {
            // 公斤数转米数
            match DualUnitConverter::kg_to_meters(req.value, req.gram_weight, req.width_cm) {
                Ok(meters) => ConvertUnitResponse {
                    converted_value: meters,
                    to_unit: "meters".to_string(),
                    formula: format!(
                        "米数 = 公斤数 ÷ (克重 (g/m²) × 幅宽 (m) ÷ 1000)\n= {} ÷ ({} × {} ÷ 1000) = {}",
                        req.value,
                        req.gram_weight,
                        req.width_cm / rust_decimal::Decimal::from(100),
                        meters
                    ),
                    conversion_rate: DualUnitConverter::calculate_conversion_rate(
                        req.gram_weight,
                        req.width_cm,
                    ).unwrap_or(rust_decimal::Decimal::ZERO),
                },
                Err(e) => return ApiResponse::<()>::error(e).into_response(),
            }
        }
        _ => return ApiResponse::<()>::error("不支持的单位转换".to_string()).into_response(),
    };

    ApiResponse::success_with_data(result).into_response()
}

/// 验证双计量单位一致性请求
#[derive(Debug, Deserialize)]
pub struct ValidateDualUnitRequest {
    /// 米数
    pub quantity_meters: rust_decimal::Decimal,
    /// 公斤数
    pub quantity_kg: rust_decimal::Decimal,
    /// 克重 (g/m²)
    pub gram_weight: rust_decimal::Decimal,
    /// 幅宽 (cm)
    pub width_cm: rust_decimal::Decimal,
    /// 允许误差率（可选，默认 0.5%）
    pub tolerance: Option<rust_decimal::Decimal>,
}

/// 验证双计量单位一致性响应
#[derive(Debug, Serialize)]
pub struct ValidateDualUnitResponse {
    /// 是否一致
    pub is_valid: bool,
    /// 计算出的公斤数
    pub calculated_kg: rust_decimal::Decimal,
    /// 差异值
    pub difference: rust_decimal::Decimal,
    /// 允许的差异值
    pub allowed_difference: rust_decimal::Decimal,
    /// 误差率
    pub error_rate: String,
}

/// 验证双计量单位一致性接口
pub async fn validate_dual_unit(Json(req): Json<ValidateDualUnitRequest>) -> impl IntoResponse {
    match DualUnitConverter::validate_dual_unit(
        req.quantity_meters,
        req.quantity_kg,
        req.gram_weight,
        req.width_cm,
        req.tolerance,
    ) {
        Ok(is_valid) => {
            // 计算详细信息
            let calculated_kg =
                DualUnitConverter::meters_to_kg(req.quantity_meters, req.gram_weight, req.width_cm)
                    .unwrap_or(rust_decimal::Decimal::ZERO);

            let difference = (calculated_kg - req.quantity_kg).abs();
            let tolerance = req.tolerance.unwrap_or(
                "0.005"
                    .parse::<rust_decimal::Decimal>()
                    .unwrap_or(rust_decimal::Decimal::ZERO),
            );
            let allowed_difference = calculated_kg * tolerance;

            let error_rate = if calculated_kg != rust_decimal::Decimal::ZERO {
                format!(
                    "{:.4}%",
                    (difference / calculated_kg) * rust_decimal::Decimal::from(100)
                )
            } else {
                "0.0000%".to_string()
            };

            let response = ValidateDualUnitResponse {
                is_valid,
                calculated_kg,
                difference,
                allowed_difference,
                error_rate,
            };

            ApiResponse::success_with_data(response).into_response()
        }
        Err(e) => ApiResponse::<()>::error(e).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    #[test]
    fn test_convert_unit_request_deserialize() {
        let json = r#"
        {
            "value": "100.000",
            "from_unit": "meters",
            "gram_weight": "180.00",
            "width_cm": "180.00"
        }
        "#;

        let req: ConvertUnitRequest =
            serde_json::from_str(json).expect("request json should deserialize");
        assert_eq!(
            req.value,
            rust_decimal::Decimal::from_str("100.000").expect("decimal should parse")
        );
        assert_eq!(req.from_unit, "meters");
        assert_eq!(
            req.gram_weight,
            rust_decimal::Decimal::from_str("180.00").expect("decimal should parse")
        );
        assert_eq!(
            req.width_cm,
            rust_decimal::Decimal::from_str("180.00").expect("decimal should parse")
        );
    }

    #[test]
    fn test_validate_dual_unit_request_deserialize() {
        let json = r#"
        {
            "quantity_meters": "100.000",
            "quantity_kg": "3.240",
            "gram_weight": "180.00",
            "width_cm": "180.00",
            "tolerance": "0.005"
        }
        "#;

        let req: ValidateDualUnitRequest =
            serde_json::from_str(json).expect("request json should deserialize");
        assert_eq!(
            req.quantity_meters,
            rust_decimal::Decimal::from_str("100.000").expect("decimal should parse")
        );
        assert_eq!(
            req.quantity_kg,
            rust_decimal::Decimal::from_str("3.240").expect("decimal should parse")
        );
        assert_eq!(
            req.gram_weight,
            rust_decimal::Decimal::from_str("180.00").expect("decimal should parse")
        );
        assert_eq!(
            req.width_cm,
            rust_decimal::Decimal::from_str("180.00").expect("decimal should parse")
        );
        assert_eq!(
            req.tolerance,
            Some(rust_decimal::Decimal::from_str("0.005").expect("decimal should parse"))
        );
    }
}
