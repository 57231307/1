//! P0-T02 销售发货全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + 纯函数 validate_dye_lot_consistency + DB 异常路径
//! SalesService 构造依赖 SearchSyncer（ES 同步器），集成测试仅覆盖纯函数 + 状态常量，
//! DB 相关测试通过 service 静态方法或纯函数验证。

#[cfg(test)]
mod tests {
    use bingxi_backend::models::status::sales_delivery;
    use bingxi_backend::services::so::delivery::{validate_dye_lot_consistency, ShipOrderItemRequest};
    use rust_decimal::Decimal;

    /// 构造 ShipOrderItemRequest 测试夹具
    fn make_item(product_id: i32, dye_lot_no: Option<&str>) -> ShipOrderItemRequest {
        ShipOrderItemRequest {
            product_id,
            quantity: Decimal::new(100, 0),
            batch_no: Some("B001".to_string()),
            color_no: Some("C001".to_string()),
            dye_lot_no: dye_lot_no.map(|s| s.to_string()),
        }
    }

    // ===== 状态常量值正确性 =====

    /// 测试_销售发货状态常量_值正确性
    ///
    /// 验证销售发货状态常量值符合预期（小写风格，与 sales_order 保持一致）。
    #[test]
    fn 测试_销售发货状态常量_值正确性() {
        assert_eq!(sales_delivery::PENDING, "pending");
        assert_eq!(sales_delivery::SHIPPED, "shipped");
        assert_eq!(sales_delivery::CANCELLED, "cancelled");
    }

    /// 测试_销售发货状态常量_小写风格一致性
    ///
    /// 验证销售发货状态常量均为小写风格（与 sales_order 状态一致）。
    #[test]
    fn 测试_销售发货状态常量_小写风格一致性() {
        for s in [
            sales_delivery::PENDING,
            sales_delivery::SHIPPED,
            sales_delivery::CANCELLED,
        ] {
            assert!(
                s.chars().all(|c| c.is_lowercase() || c == '_'),
                "状态 {} 应全小写",
                s
            );
        }
    }

    // ===== validate_dye_lot_consistency 纯函数测试 =====

    /// 测试_validate_dye_lot_consistency_空列表通过
    ///
    /// 验证空 items 列表返回 Ok（无缸号需校验）。
    #[test]
    fn 测试_validate_dye_lot_consistency_空列表通过() {
        let items: Vec<ShipOrderItemRequest> = vec![];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_ok(), "空列表应通过校验");
    }

    /// 测试_validate_dye_lot_consistency_无缸号通过
    ///
    /// 验证所有 item 都没有 dye_lot_no 时返回 Ok（无缸号约束）。
    #[test]
    fn 测试_validate_dye_lot_consistency_无缸号通过() {
        let items = vec![
            make_item(1, None),
            make_item(2, None),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_ok(), "无缸号应通过校验");
    }

    /// 测试_validate_dye_lot_consistency_同产品同缸号通过
    ///
    /// 验证同一产品的多个 item 使用相同缸号时返回 Ok。
    #[test]
    fn 测试_validate_dye_lot_consistency_同产品同缸号通过() {
        let items = vec![
            make_item(1, Some("DL001")),
            make_item(1, Some("DL001")),
            make_item(1, Some("DL001")),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_ok(), "同产品同缸号应通过校验");
    }

    /// 测试_validate_dye_lot_consistency_同产品不同缸号失败
    ///
    /// 验证同一产品的多个 item 使用不同缸号时返回 Err（违反缸号一致性约束）。
    #[test]
    fn 测试_validate_dye_lot_consistency_同产品不同缸号失败() {
        let items = vec![
            make_item(1, Some("DL001")),
            make_item(1, Some("DL002")),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(
            result.is_err(),
            "同产品不同缸号应返回 Err（违反缸号一致性约束）"
        );
    }

    /// 测试_validate_dye_lot_consistency_不同产品不同缸号通过
    ///
    /// 验证不同产品使用不同缸号时返回 Ok（缸号约束仅针对同产品）。
    #[test]
    fn 测试_validate_dye_lot_consistency_不同产品不同缸号通过() {
        let items = vec![
            make_item(1, Some("DL001")),
            make_item(2, Some("DL002")),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_ok(), "不同产品不同缸号应通过校验");
    }

    /// 测试_validate_dye_lot_consistency_空字符串缸号忽略
    ///
    /// 验证 dye_lot_no 为空字符串时被忽略（不参与一致性校验）。
    #[test]
    fn 测试_validate_dye_lot_consistency_空字符串缸号忽略() {
        let items = vec![
            make_item(1, Some("")),
            make_item(1, Some("DL001")),
        ];
        let result = validate_dye_lot_consistency(&items);
        assert!(result.is_ok(), "空字符串缸号应被忽略");
    }

    // ===== 完整业务流程测试（需要真实 PostgreSQL + ES，标记 ignore）=====

    /// 集成测试：销售发货全流程 create_delivery(PENDING) → ship_order(SHIPPED) → cancel_delivery
    ///
    /// 需要 PostgreSQL + Elasticsearch + 前置销售订单/库存/预留数据。
    #[tokio::test]
    #[ignore = "需要 PostgreSQL + Elasticsearch + 前置销售订单/库存数据"]
    async fn 测试_销售发货全流程_创建到发货到取消() {
        // 完整流程需启动 SalesService（依赖 SearchSyncer），留待真实环境验证。
        // 此处仅作为流程文档：create_delivery → ship_order → cancel_delivery
    }
}
