#[cfg(test)]
mod tests {
    use crate::handlers::inventory_stock_handler_dto::CreateStockFabricRequest;

    #[test]
    fn test_create_stock_fabric_request_deserialize() {
        let json = r#"
        {
            "warehouse_id": 1,
            "product_id": 100,
            "batch_no": "B20240101",
            "color_no": "C001",
            "dye_lot_no": "D20240101001",
            "grade": "一等品",
            "quantity_meters": "100.00",
            "gram_weight": "180.00",
            "width": "180.00",
            "location_id": 1,
            "shelf_no": "A01",
            "layer_no": "01"
        }
        "#;

        // P9-1 关键路径 unwrap 清理：单元测试中的常量 JSON 序列化使用 decs! 宏统一
        let req: CreateStockFabricRequest = serde_json::from_str(json)
            .expect("P9-1: 单元测试夹具 JSON 反序列化失败，需要排查 fixture");
        assert_eq!(req.warehouse_id, 1);
        assert_eq!(req.product_id, 100);
        assert_eq!(req.batch_no, "B20240101");
        assert_eq!(req.color_no, "C001");
        assert_eq!(req.quantity_meters, crate::decs!("100.00"));
        assert_eq!(req.gram_weight, Some(crate::decs!("180.00")));
        assert_eq!(req.width, Some(crate::decs!("180.00")));
    }
}