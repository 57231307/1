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

    /// 码↔米固定换算率（单一真源）：1 米 = 1.0936 码。
    /// 该率为行业定长换算，与克重/幅宽无关；作为报价/订单「码」单位换算视图的唯一入参，
    /// 禁止调用方各自散落字面量。米↔公斤走 meters_to_kg/kg_to_meters（克重×幅宽），二者语义不同。
    pub fn yards_per_meter() -> Decimal {
        Decimal::new(10936, 4)
    }

    /// 码数转米数：米 = 码 ÷ 1.0936（quantity_yards 码数；Ok 米数，Err 输入非法）
    pub fn yards_to_meters(quantity_yards: Decimal) -> Result<Decimal, String> {
        if quantity_yards < Decimal::ZERO {
            return Err("码数不能为负数".to_string());
        }
        Ok((quantity_yards / Self::yards_per_meter()).round_dp(2))
    }

    /// 匹数转米数：米 = 匹 × 每匹米数（piece_count 匹数，meters_per_piece 取自产品换算元数据列）
    pub fn pieces_to_meters(
        piece_count: Decimal,
        meters_per_piece: Decimal,
    ) -> Result<Decimal, String> {
        if piece_count < Decimal::ZERO {
            return Err("匹数不能为负数".to_string());
        }
        if meters_per_piece <= Decimal::ZERO {
            return Err("每匹米数必须大于 0".to_string());
        }
        Ok((piece_count * meters_per_piece).round_dp(2))
    }

    /// 卷数转米数：米 = 卷 × 每卷米数（roll_count 卷数，meters_per_roll 取自产品换算元数据列）
    pub fn rolls_to_meters(
        roll_count: Decimal,
        meters_per_roll: Decimal,
    ) -> Result<Decimal, String> {
        if roll_count < Decimal::ZERO {
            return Err("卷数不能为负数".to_string());
        }
        if meters_per_roll <= Decimal::ZERO {
            return Err("每卷米数必须大于 0".to_string());
        }
        Ok((roll_count * meters_per_roll).round_dp(2))
    }
}
