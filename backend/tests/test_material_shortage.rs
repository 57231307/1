//! 缺料预警 Service 单元测试

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn test_shortage_level_from_deficit_rate() {
        // 测试预警级别判断
        use bingxi_backend::services::material_shortage_service::ShortageLevel;

        // 缺口100%应该是Critical
        assert_eq!(
            ShortageLevel::from_deficit_rate(Decimal::from(100)),
            ShortageLevel::Critical
        );

        // 缺口80%应该是Severe
        assert_eq!(
            ShortageLevel::from_deficit_rate(Decimal::from(80)),
            ShortageLevel::Severe
        );

        // 缺口30%应该是Warning
        assert_eq!(
            ShortageLevel::from_deficit_rate(Decimal::from(30)),
            ShortageLevel::Warning
        );

        // 缺口0%应该是Normal
        assert_eq!(
            ShortageLevel::from_deficit_rate(Decimal::from(0)),
            ShortageLevel::Normal
        );
    }

    #[test]
    fn test_material_shortage_item_creation() {
        // 测试缺料项创建
        let item = bingxi_backend::services::material_shortage_service::MaterialShortageItem {
            material_id: 1,
            material_name: "棉布A".to_string(),
            material_code: "M001".to_string(),
            required_quantity: Decimal::from(1000),
            available_quantity: Decimal::from(500),
            shortage_quantity: Decimal::from(500),
            deficit_rate: Decimal::from(50),
            level: bingxi_backend::services::material_shortage_service::ShortageLevel::Severe,
            affected_orders: vec![],
            unit: Some("米".to_string()),
        };

        assert_eq!(item.material_id, 1);
        assert_eq!(item.shortage_quantity, Decimal::from(500));
    }

    #[test]
    fn test_affected_order_creation() {
        // 测试受影响订单创建
        let order = bingxi_backend::services::material_shortage_service::AffectedOrder {
            order_id: 1,
            order_no: "PO-001".to_string(),
            demand_quantity: Decimal::from(500),
            planned_end_date: Some(NaiveDate::from_ymd_opt(2026, 5, 25).unwrap()),
        };

        assert_eq!(order.order_id, 1);
        assert_eq!(order.order_no, "PO-001");
    }

    #[test]
    fn test_shortage_threshold_config_default() {
        // 测试默认阈值配置
        let config =
            bingxi_backend::services::material_shortage_service::ShortageThresholdConfig::default();

        assert_eq!(config.safety_factor, Decimal::from(1));
        assert_eq!(config.critical_threshold, Decimal::from(100));
        assert_eq!(config.severe_threshold, Decimal::from(50));
    }

    #[test]
    fn test_replenishment_suggestion_priority_order() {
        // 测试补货建议优先级排序
        let mut suggestions = [
            bingxi_backend::services::material_shortage_service::ReplenishmentSuggestion {
                material_id: 1,
                material_name: "材料A".to_string(),
                material_code: "M001".to_string(),
                shortage_quantity: Decimal::from(100),
                suggested_quantity: Decimal::from(120),
                unit: Some("个".to_string()),
                priority: "LOW".to_string(),
                affected_orders_count: 1,
            },
            bingxi_backend::services::material_shortage_service::ReplenishmentSuggestion {
                material_id: 2,
                material_name: "材料B".to_string(),
                material_code: "M002".to_string(),
                shortage_quantity: Decimal::from(200),
                suggested_quantity: Decimal::from(240),
                unit: Some("个".to_string()),
                priority: "URGENT".to_string(),
                affected_orders_count: 3,
            },
        ];

        // 按优先级排序
        suggestions.sort_by(|a, b| {
            let priority_order = |p: &str| match p {
                "URGENT" => 0,
                "HIGH" => 1,
                "MEDIUM" => 2,
                _ => 3,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
        });

        assert_eq!(suggestions[0].priority, "URGENT");
        assert_eq!(suggestions[1].priority, "LOW");
    }
}
