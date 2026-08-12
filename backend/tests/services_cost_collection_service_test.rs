#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use super::*;

    #[test]
    fn test_cost_ratio_calculation() {
        // P9-1: 用 P9-1 标记的 helper 解析测试夹具 Decimal
        let dec_from = |f: f64| -> Decimal {
            Decimal::try_from(f).expect("P9-1: 测试夹具 Decimal::try_from 失败")
        };

        let total_cost = Decimal::from(10000);
        let direct_material = Decimal::from(5000);
        let direct_labor = Decimal::from(2000);
        let manufacturing_overhead = Decimal::from(3000);

        // 计算各项占比
        let material_ratio = direct_material / total_cost;
        let labor_ratio = direct_labor / total_cost;
        let overhead_ratio = manufacturing_overhead / total_cost;

        assert_eq!(material_ratio, dec_from(0.5));
        assert_eq!(labor_ratio, dec_from(0.2));
        assert_eq!(overhead_ratio, dec_from(0.3));

        // 验证占比之和为 1
        let total_ratio = material_ratio + labor_ratio + overhead_ratio;
        assert_eq!(total_ratio, Decimal::ONE);
    }

    #[test]
    fn test_cost_analysis_summary_fields() {
        // P9-1: 用 helper 集中处理 Decimal::try_from
        let dec_from = |f: f64| -> Decimal {
            Decimal::try_from(f).expect("P9-1: 测试夹具 Decimal::try_from 失败")
        };
        let summary = CostAnalysisSummary {
            record_count: 10,
            total_direct_material: Decimal::from(50000),
            total_direct_labor: Decimal::from(20000),
            total_overhead: Decimal::from(15000),
            total_processing: Decimal::from(8000),
            total_dyeing: Decimal::from(7000),
            total_cost: Decimal::from(100000),
            total_output_meters: Decimal::from(5000),
            total_output_kg: Decimal::from(2000),
            avg_unit_cost_meters: Some(Decimal::from(20)),
            avg_unit_cost_kg: Some(Decimal::from(50)),
            material_ratio: Some(dec_from(0.5)),
            labor_ratio: Some(dec_from(0.2)),
            overhead_ratio: Some(dec_from(0.15)),
        };

        assert_eq!(summary.record_count, 10);
        assert_eq!(summary.total_cost, Decimal::from(100000));
        assert!(summary.avg_unit_cost_meters.is_some());
    }
}