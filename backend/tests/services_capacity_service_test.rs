#[cfg(test)]
mod tests {
    use bingxi_backend::services::capacity_service::CapacityService;

    /// 无历史数据时置信度应较低
    #[test]
    fn test_confidence_no_history() {
        let confidence = CapacityService::calculate_forecast_confidence(0, 30, true);
        // 基础 0.30 + 当前负荷 0.05 = 0.35，期限因子 0.92 → 0.322
        assert!(
            confidence < 0.40,
            "无历史数据置信度应低于 0.40，实际: {}",
            confidence
        );
    }

    /// 历史数据丰富且短期预测时置信度应较高
    #[test]
    fn test_confidence_rich_history_short_horizon() {
        let confidence = CapacityService::calculate_forecast_confidence(100, 7, true);
        // 基础 0.85 + 当前负荷 0.05 = 0.90，期限因子 1.00 → 0.90
        assert!(
            confidence >= 0.85,
            "丰富历史+短期预测置信度应 >= 0.85，实际: {}",
            confidence
        );
    }

    /// 长期预测置信度应低于短期预测
    #[test]
    fn test_confidence_long_horizon_lower() {
        let short_confidence = CapacityService::calculate_forecast_confidence(30, 7, true);
        let long_confidence = CapacityService::calculate_forecast_confidence(30, 365, true);
        assert!(
            long_confidence < short_confidence,
            "长期预测置信度 ({}) 应低于短期 ({})",
            long_confidence,
            short_confidence
        );
    }

    /// 有当前负荷数据时置信度应高于无负荷数据
    #[test]
    fn test_confidence_current_load_bonus() {
        let with_load = CapacityService::calculate_forecast_confidence(10, 30, true);
        let without_load = CapacityService::calculate_forecast_confidence(10, 30, false);
        assert!(
            with_load > without_load,
            "有当前负荷数据置信度 ({}) 应高于无负荷 ({})",
            with_load,
            without_load
        );
    }

    /// 置信度应始终在 [0.10, 0.95] 区间内
    #[test]
    fn test_confidence_within_bounds() {
        // 最差情况：无历史 + 无负荷 + 超长期
        let min_confidence = CapacityService::calculate_forecast_confidence(0, 365, false);
        assert!(
            min_confidence >= 0.10,
            "置信度下限应 >= 0.10，实际: {}",
            min_confidence
        );

        // 最好情况：丰富历史 + 有负荷 + 短期
        let max_confidence = CapacityService::calculate_forecast_confidence(1000, 1, true);
        assert!(
            max_confidence <= 0.95,
            "置信度上限应 <= 0.95，实际: {}",
            max_confidence
        );
    }
}