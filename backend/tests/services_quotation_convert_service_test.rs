    use super::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_compose_color_no_empty() {
        let item = sales_quotation_item::Model {
            id: 1,
            quotation_id: 1,
            product_id: 1,
            color_id: None,
            color_code: None,
            pantone_code: None,
            cncs_code: None,
            specification: None,
            unit: "米".to_string(),
            quantity: Decimal::from(10),
            unit_price: Decimal::from(10),
            unit_price_with_tax: Decimal::from(11),
            amount: Decimal::from(100),
            amount_with_tax: Decimal::from(113),
            tier_pricing: None,
            discount_rate: None,
            discount_amount: None,
            notes: None,
            sequence: 0,
        };
        assert_eq!(QuotationConvertService::compose_color_no(&item), "-");
    }

    #[test]
    fn test_compose_color_no_with_pantone() {
        let item = sales_quotation_item::Model {
            id: 1,
            quotation_id: 1,
            product_id: 1,
            color_id: None,
            color_code: Some("RED-01".to_string()),
            pantone_code: Some("18-1664".to_string()),
            cncs_code: None,
            specification: None,
            unit: "米".to_string(),
            quantity: Decimal::from(10),
            unit_price: Decimal::from(10),
            unit_price_with_tax: Decimal::from(11),
            amount: Decimal::from(100),
            amount_with_tax: Decimal::from(113),
            tier_pricing: None,
            discount_rate: None,
            discount_amount: None,
            notes: None,
            sequence: 0,
        };
        let s = QuotationConvertService::compose_color_no(&item);
        assert!(s.contains("RED-01"));
        assert!(s.contains("PANTONE:18-1664"));
    }
}