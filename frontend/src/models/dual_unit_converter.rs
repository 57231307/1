//! 双计量单位转换模型
//!
//! 双计量单位转换相关的数据结构

use serde::{Deserialize, Serialize};

/// 单位换算请求
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
