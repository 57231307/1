use axum::Json;
use serde::{Deserialize, Serialize};

use crate::utils::ApiResponse;
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;

/// 单位换算请求
#[allow(dead_code, reason = "反序列化输入字段")]
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
#[allow(dead_code, reason = "序列化输出字段")]
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

/// 双计量单位换算接口（米↔公斤）
pub async fn convert_dual_unit(
    Json(req): Json<ConvertUnitRequest>,
) -> Result<Json<ApiResponse<ConvertUnitResponse>>, AppError> {
    // 验证单位参数
    let from_unit = req.from_unit.to_lowercase();
    if from_unit != "meters" && from_unit != "kg" {
        return Err(AppError::bad_request("无效的单位，必须是 'meters' 或 'kg'"));
    }

    // 执行换算
    let result = match from_unit.as_str() {
        "meters" => {
            // 米数转公斤数
            let kg = DualUnitConverter::meters_to_kg(req.value, req.gram_weight, req.width_cm)
                .map_err(AppError::bad_request)?;
            ConvertUnitResponse {
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
            }
        }
        "kg" => {
            // 公斤数转米数
            let meters = DualUnitConverter::kg_to_meters(req.value, req.gram_weight, req.width_cm)
                .map_err(AppError::bad_request)?;
            ConvertUnitResponse {
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
                )
                .unwrap_or(rust_decimal::Decimal::ZERO),
            }
        }
        // 批次 252 修复：原 unreachable!() 在校验逻辑被重构后可能 panic 崩溃，
        // 改为防御性返回 bad_request 错误
        _ => return Err(AppError::bad_request("无效的单位，必须是 'meters' 或 'kg'")),
    };

    Ok(Json(ApiResponse::success(result)))
}

/// 验证双计量单位一致性请求
#[allow(dead_code, reason = "反序列化输入字段")]
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
#[allow(dead_code, reason = "序列化输出字段")]
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

/// 验证双计量单位一致性接口；BE-A/H 统一（2026-06-26）：返回类型从 impl IntoResponse 改为 Result<Json<ApiResponse<T>>, AppError>。
pub async fn validate_dual_unit(
    Json(req): Json<ValidateDualUnitRequest>,
) -> Result<Json<ApiResponse<ValidateDualUnitResponse>>, AppError> {
    let is_valid = DualUnitConverter::validate_dual_unit(
        req.quantity_meters,
        req.quantity_kg,
        req.gram_weight,
        req.width_cm,
        req.tolerance,
    )
    .map_err(AppError::bad_request)?;

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

    Ok(Json(ApiResponse::success(response)))
}
