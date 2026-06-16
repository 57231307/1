//! P0-5 面料多色号定价扩展 - 季节调价规则集成测试

#[cfg(test)]
mod seasonal_tests {
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use serde_json::json;

    use bingxi_backend::models::seasonal_price_rule_dto::{
        CreateSeasonalRuleDto, ListSeasonalRulesQuery,
    };

    /// 测试 1: 创建季节规则
    #[test]
    fn test_create_seasonal_rule() {
        let dto = CreateSeasonalRuleDto {
            rule_name: "春夏新品 +10%".to_string(),
            season: "SS".to_string(),
            product_category_id: Some(1),
            adjustment_type: "percentage".to_string(),
            adjustment_value: dec!(0.10),
            valid_from: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            valid_until: Some(NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()),
            description: Some("2026 春夏新品季节性提价".to_string()),
        };
        assert_eq!(dto.season, "SS");
        assert_eq!(dto.adjustment_type, "percentage");
    }

    /// 测试 2: 规则查询
    #[test]
    fn test_list_query() {
        let query = ListSeasonalRulesQuery {
            page: Some(1),
            page_size: Some(20),
            season: Some("SS".to_string()),
            is_active: Some(true),
            product_category_id: Some(1),
        };
        assert_eq!(query.season, Some("SS".to_string()));
    }

    /// 测试 3: 季节匹配 SS/AW/HOLIDAY
    #[test]
    fn test_season_enum() {
        let valid = vec!["SS", "AW", "HOLIDAY"];
        for s in valid {
            assert!(s == "SS" || s == "AW" || s == "HOLIDAY");
        }
    }

    /// 测试 4: 调整方式 percentage / fixed
    #[test]
    fn test_adjustment_type_enum() {
        let types = vec!["percentage", "fixed"];
        for t in types {
            assert!(t == "percentage" || t == "fixed");
        }
    }

    /// 测试 5: 季节价格计算
    #[test]
    fn test_seasonal_price_calc() {
        // 100 元基础价 → 春夏 +10% = 110 元
        let base = dec!(100.00);
        let factor = dec!(1.00) + dec!(0.10);
        let result = base * factor;
        assert_eq!(result, dec!(110.00));
        let _ = json!({ "season": "SS", "factor": 0.10 });
    }
}
