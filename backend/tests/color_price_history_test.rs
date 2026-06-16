//! P0-5 面料多色号定价扩展 - 价格历史集成测试

#[cfg(test)]
mod history_tests {
    use rust_decimal_macros::dec;

    use bingxi_backend::models::color_price_history_dto::{PriceHistoryItem, PriceHistoryQuery};

    /// 测试 1: 价格历史结构
    #[test]
    fn test_history_item_structure() {
        let item = PriceHistoryItem {
            id: 1,
            product_color_price_id: 100,
            old_price: dec!(50.00),
            new_price: dec!(55.00),
            currency: "CNY".to_string(),
            change_type: "manual".to_string(),
            change_reason: Some("成本上涨".to_string()),
            change_percent: Some(dec!(0.10)),
            quantity: None,
            operated_by: 1,
            operated_at: chrono::Utc::now(),
            approved_by: Some(2),
            approved_at: Some(chrono::Utc::now()),
        };
        assert_eq!(item.old_price, dec!(50.00));
        assert_eq!(item.new_price, dec!(55.00));
        assert_eq!(item.change_percent, Some(dec!(0.10)));
    }

    /// 测试 2: 历史查询参数
    #[test]
    fn test_history_query() {
        let query = PriceHistoryQuery {
            page: Some(1),
            page_size: Some(50),
            product_id: Some(100),
            color_id: Some(200),
            from_date: None,
            to_date: None,
            change_type: Some("batch".to_string()),
        };
        assert_eq!(query.product_id, Some(100));
        assert_eq!(query.change_type, Some("batch".to_string()));
    }

    /// 测试 3: 涨跌幅计算
    #[test]
    fn test_change_percent_calculation() {
        let old = dec!(50.00);
        let new = dec!(55.00);
        let change = (new - old) / old;
        assert_eq!(change, dec!(0.10));
    }
}
