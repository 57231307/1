#[cfg(test)]
mod tests {
    use bingxi_backend::models::color_price_dto::*;
    use rust_decimal::Decimal;
    use serde_json::json;

    // ===== DTO 测试 =====

    #[test]
    fn test_color_price_create_dto() {
        let dto = CreateColorPriceDto {
            product_id: 1,
            color_id: 1,
            price: Decimal::new(1000, 2),
            currency: Some("CNY".to_string()),
            unit: Some("米".to_string()),
            min_quantity: Some(Decimal::new(100, 0)),
            max_quantity: Some(Decimal::new(1000, 0)),
            effective_date: Some("2026-01-01".to_string()),
            expiry_date: Some("2026-12-31".to_string()),
            remark: Some("测试备注".to_string()),
        };

        assert_eq!(dto.product_id, 1);
        assert_eq!(dto.color_id, 1);
        assert_eq!(dto.price, Decimal::new(1000, 2));
    }

    #[test]
    fn test_color_price_update_dto() {
        let dto = UpdateColorPriceDto {
            price: Some(Decimal::new(1200, 2)),
            currency: Some("USD".to_string()),
            min_quantity: Some(Decimal::new(50, 0)),
            max_quantity: Some(Decimal::new(500, 0)),
            effective_date: Some("2026-02-01".to_string()),
            expiry_date: Some("2026-12-31".to_string()),
            remark: Some("更新备注".to_string()),
        };

        assert_eq!(dto.price, Some(Decimal::new(1200, 2)));
    }

    #[test]
    fn test_color_price_query_dto() {
        let dto = ColorPriceQueryDto {
            product_id: Some(1),
            color_id: Some(1),
            min_price: Some(Decimal::new(500, 2)),
            max_price: Some(Decimal::new(2000, 2)),
            page: Some(1),
            page_size: Some(10),
        };

        assert_eq!(dto.product_id, Some(1));
        assert_eq!(dto.page, Some(1));
    }

    // ===== 序列化测试 =====

    #[test]
    fn test_create_color_price_dto_serialization() {
        let dto = CreateColorPriceDto {
            product_id: 1,
            color_id: 1,
            price: Decimal::new(1000, 2),
            currency: Some("CNY".to_string()),
            unit: Some("米".to_string()),
            min_quantity: Some(Decimal::new(100, 0)),
            max_quantity: Some(Decimal::new(1000, 0)),
            effective_date: Some("2026-01-01".to_string()),
            expiry_date: Some("2026-12-31".to_string()),
            remark: Some("测试备注".to_string()),
        };

        let json = serde_json::to_value(&dto).expect("序列化失败");
        assert_eq!(json["product_id"], 1);
        assert_eq!(json["price"], "10.00");
    }

    #[test]
    fn test_color_price_query_dto_deserialization() {
        let json = json!({
            "product_id": 1,
            "color_id": 1,
            "min_price": "5.00",
            "max_price": "20.00",
            "page": 1,
            "page_size": 10
        });

        let dto: ColorPriceQueryDto = serde_json::from_value(json).expect("反序列化失败");
        assert_eq!(dto.product_id, Some(1));
        assert_eq!(dto.page, Some(1));
    }

    // ===== 价格计算测试 =====

    #[test]
    fn test_price_with_quantity_discount() {
        let base_price = Decimal::new(1000, 2);
        let quantity = Decimal::new(500, 0);
        let discount_rate = Decimal::new(90, 2); // 90%
        let final_price = base_price * discount_rate;

        assert_eq!(final_price, Decimal::new(900, 2));
    }

    #[test]
    fn test_price_range() {
        let min_price = Decimal::new(500, 2);
        let max_price = Decimal::new(2000, 2);
        let actual_price = Decimal::new(1000, 2);

        // 验证价格在范围内
        assert!(actual_price >= min_price);
        assert!(actual_price <= max_price);
    }
}
