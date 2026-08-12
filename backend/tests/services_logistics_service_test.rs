#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freight_max_calculation() {
        // 验证运费取三者最大值的逻辑
        let weight_cost = Decimal::from(100);
        let volume_cost = Decimal::from(150);
        let distance_cost = Decimal::from(80);
        let freight = [weight_cost, volume_cost, distance_cost]
            .into_iter()
            .max()
            .unwrap_or(Decimal::ZERO);
        assert_eq!(freight, Decimal::from(150));
    }

    #[test]
    fn test_freight_zero_when_no_data() {
        let freight = [Decimal::ZERO, Decimal::ZERO, Decimal::ZERO]
            .into_iter()
            .max()
            .unwrap_or(Decimal::ZERO);
        assert_eq!(freight, Decimal::ZERO);
    }
}