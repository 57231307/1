/// 双计量单位换算工具（面料行业专用）
///
/// 提供米数 ↔ 公斤数的精确换算功能
/// 支持幅宽、克重等多参数计算
use rust_decimal::Decimal;
use std::str::FromStr;

/// 双计量单位换算器
pub struct DualUnitConverter;

/// 换算结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConversionResult {
    pub original_quantity: Decimal,
    pub original_unit: String,
    pub converted_quantity: Decimal,
    pub converted_unit: String,
    pub conversion_rate: Decimal,
    pub formula: String,
}

impl DualUnitConverter {
    /// 米数转公斤数（精确版）
    ///
    /// 公式：公斤数 = 米数 × 克重 (g/m²) × 幅宽 (m) ÷ 1000
    ///
    /// # Arguments
    /// * `quantity_meters` - 米数
    /// * `gram_weight` - 克重（g/m²）
    /// * `width_cm` - 幅宽（cm）
    ///
    /// # Returns
    /// * `Ok(Decimal)` - 公斤数
    /// * `Err(String)` - 错误信息
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

    /// 公斤数转米数（精确版）
    ///
    /// 公式：米数 = 公斤数 × 1000 ÷ 克重 (g/m²) ÷ 幅宽 (m)
    ///
    /// # Arguments
    /// * `quantity_kg` - 公斤数
    /// * `gram_weight` - 克重（g/m²）
    /// * `width_cm` - 幅宽（cm）
    ///
    /// # Returns
    /// * `Ok(Decimal)` - 米数
    /// * `Err(String)` - 错误信息
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

    /// 自动换算（根据输入单位智能判断）
    ///
    /// # Arguments
    /// * `quantity` - 数量
    /// * `unit` - 单位（"米" 或 "公斤"）
    /// * `gram_weight` - 克重（g/m²）
    /// * `width_cm` - 幅宽（cm）
    ///
    /// # Returns
    /// * `Ok(ConversionResult)` - 换算结果
    /// * `Err(String)` - 错误信息
    #[allow(dead_code)]
    pub fn auto_convert(
        quantity: Decimal,
        unit: &str,
        gram_weight: Decimal,
        width_cm: Decimal,
    ) -> Result<ConversionResult, String> {
        match unit {
            "米" | "M" | "meters" | "meter" => {
                let kg = Self::meters_to_kg(quantity, gram_weight, width_cm)?;
                let rate = kg / quantity;
                let formula = format!(
                    "{}米 × {}g/m² × {}m ÷ 1000 = {}公斤",
                    quantity,
                    gram_weight,
                    width_m_str(&width_cm),
                    kg
                );

                Ok(ConversionResult {
                    original_quantity: quantity,
                    original_unit: "米".to_string(),
                    converted_quantity: kg,
                    converted_unit: "公斤".to_string(),
                    conversion_rate: rate,
                    formula,
                })
            }
            "公斤" | "KG" | "kg" | "kilogram" => {
                let meters = Self::kg_to_meters(quantity, gram_weight, width_cm)?;
                let rate = quantity / meters;
                let formula = format!(
                    "{}公斤 × 1000 ÷ {}g/m² ÷ {}m = {}米",
                    quantity,
                    gram_weight,
                    width_m_str(&width_cm),
                    meters
                );

                Ok(ConversionResult {
                    original_quantity: quantity,
                    original_unit: "公斤".to_string(),
                    converted_quantity: meters,
                    converted_unit: "米".to_string(),
                    conversion_rate: rate,
                    formula,
                })
            }
            _ => Err(format!("不支持的单位：{}（支持的单位：米、公斤）", unit)),
        }
    }

    /// 验证双计量单位一致性
    ///
    /// # Arguments
    /// * `quantity_meters` - 米数
    /// * `quantity_kg` - 公斤数
    /// * `gram_weight` - 克重（g/m²）
    /// * `width_cm` - 幅宽（cm）
    /// * `tolerance` - 允许误差（默认 0.5%）
    ///
    /// # Returns
    /// * `Ok(bool)` - 是否一致
    /// * `Err(String)` - 错误信息
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

/// 辅助函数：将幅宽从 cm 转换为 m 并格式化为字符串
#[allow(dead_code)]
fn width_m_str(width_cm: &Decimal) -> String {
    let width_m = width_cm / Decimal::from(100);
    width_m.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meters_to_kg_basic() {
        let quantity = Decimal::from_f64_retain(1000.0).unwrap(); // 1000 米
        let gram_weight = Decimal::from_f64_retain(170.0).unwrap(); // 170g/m²
        let width = Decimal::from_f64_retain(180.0).unwrap(); // 180cm

        let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width).unwrap();

        // 预期：1000 × 170 × 1.8 ÷ 1000 = 306 公斤
        assert_eq!(result, Decimal::from_f64_retain(306.0).unwrap());
    }

    #[test]
    fn test_kg_to_meters_basic() {
        let quantity = Decimal::from_f64_retain(306.0).unwrap(); // 306 公斤
        let gram_weight = Decimal::from_f64_retain(170.0).unwrap(); // 170g/m²
        let width = Decimal::from_f64_retain(180.0).unwrap(); // 180cm

        let result = DualUnitConverter::kg_to_meters(quantity, gram_weight, width).unwrap();

        // 预期：306 × 1000 ÷ 170 ÷ 1.8 = 1000 米
        assert_eq!(result, Decimal::from_f64_retain(1000.0).unwrap());
    }

    #[test]
    fn test_auto_convert_meters_to_kg() {
        let quantity = Decimal::from_f64_retain(5000.0).unwrap();
        let gram_weight = Decimal::from_f64_retain(170.0).unwrap();
        let width = Decimal::from_f64_retain(180.0).unwrap();

        let result = DualUnitConverter::auto_convert(quantity, "米", gram_weight, width).unwrap();

        assert_eq!(result.converted_unit, "公斤");
        // 5000 × 170 × 1.8 ÷ 1000 = 1530 公斤
        assert_eq!(
            result.converted_quantity,
            Decimal::from_f64_retain(1530.0).unwrap()
        );
    }

    #[test]
    fn test_validate_dual_unit_valid() {
        let quantity_meters = Decimal::from_f64_retain(1000.0).unwrap();
        let quantity_kg = Decimal::from_f64_retain(306.0).unwrap();
        let gram_weight = Decimal::from_f64_retain(170.0).unwrap();
        let width = Decimal::from_f64_retain(180.0).unwrap();

        let is_valid = DualUnitConverter::validate_dual_unit(
            quantity_meters,
            quantity_kg,
            gram_weight,
            width,
            None,
        )
        .unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_validate_dual_unit_invalid() {
        let quantity_meters = Decimal::from_f64_retain(1000.0).unwrap();
        let quantity_kg = Decimal::from_f64_retain(350.0).unwrap(); // 错误的公斤数
        let gram_weight = Decimal::from_f64_retain(170.0).unwrap();
        let width = Decimal::from_f64_retain(180.0).unwrap();

        let is_valid = DualUnitConverter::validate_dual_unit(
            quantity_meters,
            quantity_kg,
            gram_weight,
            width,
            None,
        )
        .unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_negative_quantity_should_fail() {
        let quantity = Decimal::from_f64_retain(-100.0).unwrap();
        let gram_weight = Decimal::from_f64_retain(170.0).unwrap();
        let width = Decimal::from_f64_retain(180.0).unwrap();

        let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "米数不能为负数");
    }

    #[test]
    fn test_zero_gram_weight_should_fail() {
        let quantity = Decimal::from_f64_retain(1000.0).unwrap();
        let gram_weight = Decimal::from_f64_retain(0.0).unwrap();
        let width = Decimal::from_f64_retain(180.0).unwrap();

        let result = DualUnitConverter::meters_to_kg(quantity, gram_weight, width);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "克重必须大于 0");
    }
}
