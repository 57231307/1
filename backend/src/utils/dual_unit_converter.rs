/// 双计量单位换算工具（面料行业专用，提供米数↔公斤数精确换算，支持幅宽/克重等多参数）
use rust_decimal::Decimal;
use std::str::FromStr;

/// 双计量单位换算器
pub struct DualUnitConverter;

impl DualUnitConverter {
    /// 米数转公斤数（精确版）：公斤数 = 米数 × 克重(g/m²) × 幅宽(m) ÷ 1000；quantity_meters 米数，gram_weight 克重，width_cm 幅宽(cm)；Ok(Decimal) 公斤数，Err(String) 错误
    pub fn meters_to_kg(
        quantity_meters: Decimal,
        gram_weight: Decimal,
        width_cm: Decimal,
    ) -> Result<Decimal, String> {
        // 参数验证
        if quantity_meters < Decimal::ZERO {
            return Err("米数不能为负数".to_string());
        }
        if gram_weight <= Decimal::ZERO {
            return Err("克重必须大于 0".to_string());
        }
        if width_cm <= Decimal::ZERO {
            return Err("幅宽必须大于 0".to_string());
        }

        // 计算：米数 × 克重 × 幅宽 (m) ÷ 1000
        let width_m = width_cm / Decimal::from(100);
        let quantity_kg = quantity_meters * gram_weight * width_m / Decimal::from(1000);

        // 保留 3 位小数
        Ok(quantity_kg.round_dp(3))
    }

    /// 公斤数转米数（精确版）：米数 = 公斤数 × 1000 ÷ 克重(g/m²) ÷ 幅宽(m)；quantity_kg 公斤数，gram_weight 克重，width_cm 幅宽(cm)；Ok(Decimal) 米数，Err(String) 错误
    pub fn kg_to_meters(
        quantity_kg: Decimal,
        gram_weight: Decimal,
        width_cm: Decimal,
    ) -> Result<Decimal, String> {
        // 参数验证
        if quantity_kg < Decimal::ZERO {
            return Err("公斤数不能为负数".to_string());
        }
        if gram_weight <= Decimal::ZERO {
            return Err("克重必须大于 0".to_string());
        }
        if width_cm <= Decimal::ZERO {
            return Err("幅宽必须大于 0".to_string());
        }

        // 计算：公斤数 × 1000 ÷ 克重 ÷ 幅宽 (m)
        let width_m = width_cm / Decimal::from(100);
        let quantity_meters = quantity_kg * Decimal::from(1000) / (gram_weight * width_m);

        // 保留 2 位小数
        Ok(quantity_meters.round_dp(2))
    }

    /// 验证双计量单位一致性（quantity_meters 米数，quantity_kg 公斤数，gram_weight 克重，width_cm 幅宽，tolerance 允许误差默认 0.5%；Ok(bool) 是否一致，Err(String) 错误）
    pub fn validate_dual_unit(
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        gram_weight: Decimal,
        width_cm: Decimal,
        tolerance: Option<Decimal>,
    ) -> Result<bool, String> {
        let calculated_kg = Self::meters_to_kg(quantity_meters, gram_weight, width_cm)?;

        // 默认允许 0.5% 的误差
        let tolerance =
            tolerance.unwrap_or(Decimal::from_str("0.005").unwrap_or(Decimal::new(5, 3)));
        let diff = (calculated_kg - quantity_kg).abs();
        let allowed_diff = calculated_kg * tolerance;

        Ok(diff <= allowed_diff)
    }

    /// 计算换算率（每公斤多少米）
    pub fn calculate_conversion_rate(
        gram_weight: Decimal,
        width_cm: Decimal,
    ) -> Result<Decimal, String> {
        if gram_weight <= Decimal::ZERO {
            return Err("克重必须大于 0".to_string());
        }
        if width_cm <= Decimal::ZERO {
            return Err("幅宽必须大于 0".to_string());
        }

        let width_m = width_cm / Decimal::from(100);
        // 1 公斤 = 1000 ÷ 克重 ÷ 幅宽 (m) 米
        let rate = Decimal::from(1000) / (gram_weight * width_m);

        Ok(rate.round_dp(4))
    }
}
