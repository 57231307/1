//! P0-T02 采购收货全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径 + 完整流程（#[ignore]）
//! COMPLETED 状态当前在 PurchaseReceiptService 中无公开方法触发，
//! 集成测试覆盖到 CONFIRMED 流转即可（DRAFT → CONFIRMED）。

mod common;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bingxi_backend::models::status::purchase_receipt;
    use bingxi_backend::services::purchase_receipt_dto::CreatePurchaseReceiptRequest;
    use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
    use sea_orm::{Database, DatabaseConnection};
    use common::setup_test_db;

    /// 构造最小 CreatePurchaseReceiptRequest（仅必填字段）
    fn sample_request() -> CreatePurchaseReceiptRequest {
        use bingxi_backend::services::purchase_receipt_dto::CreateReceiptItemRequest;
        use rust_decimal::Decimal;

        CreatePurchaseReceiptRequest {
            receipt_no: None,
            order_id: Some(1),
            supplier_id: 1,
            warehouse_id: 1,
            receipt_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            remarks: None,
            items: vec![CreateReceiptItemRequest {
                order_item_id: Some(1),
                material_id: 1,
                quantity: Decimal::new(100, 0),
                quantity_alt: Some(Decimal::new(50, 0)),
                unit_price: Some(Decimal::new(10, 0)),
                batch_no: Some("B001".to_string()),
                dye_lot_no: None,
                remarks: None,
            }],
        }
    }

    // ===== 状态常量值正确性 =====

    /// 测试_采购收货状态常量_值正确性
    ///
    /// 验证采购收货状态常量值符合预期（大写风格）。
    #[test]
    fn 测试_采购收货状态常量_值正确性() {
        assert_eq!(purchase_receipt::DRAFT, "DRAFT");
        assert_eq!(purchase_receipt::CONFIRMED, "CONFIRMED");
        assert_eq!(purchase_receipt::COMPLETED, "COMPLETED");
    }

    /// 测试_采购收货状态常量_大写风格一致性
    ///
    /// 验证采购收货状态常量均为大写 + 下划线风格。
    #[test]
    fn 测试_采购收货状态常量_大写风格一致性() {
        for s in [
            purchase_receipt::DRAFT,
            purchase_receipt::CONFIRMED,
            purchase_receipt::COMPLETED,
        ] {
            assert!(
                s.chars().all(|c| c.is_uppercase() || c == '_'),
                "状态 {} 应全大写",
                s
            );
        }
    }

    // ===== Service 实例化与 DB 异常路径 =====

    /// 测试_PurchaseReceiptService_实例化不触发DB
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_实例化不触发DB() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let _ = svc;
    }

    /// 测试_PurchaseReceiptService_create_receipt_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 create_receipt 方法返回 Err 而非 panic。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_create_receipt_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = sample_request();
        let result = svc.create_receipt(req, 1).await;
        assert!(
            result.is_err(),
            "空 DB 上 create_receipt 应返回 Err，实际：{:?}",
            result
        );
    }

    /// 测试_PurchaseReceiptService_confirm_receipt_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 confirm_receipt 方法返回 Err 而非 panic。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_confirm_receipt_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.confirm_receipt(1, 1).await;
        assert!(
            result.is_err(),
            "空 DB 上 confirm_receipt 应返回 Err"
        );
    }

    /// 测试_PurchaseReceiptService_get_receipt_空DB返回Err
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_get_receipt_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.get_receipt(1).await;
        assert!(result.is_err(), "空 DB 上 get_receipt 应返回 Err");
    }

    /// 测试_PurchaseReceiptService_list_receipts_空DB返回Err
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_list_receipts_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.list_receipts(None, None, 1, 20).await;
        assert!(
            result.is_err(),
            "空 DB 上 list_receipts 应返回 Err"
        );
    }

    // ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

    /// 集成测试：采购收货全流程 create(DRAFT) → confirm(CONFIRMED)
    ///
    /// 需要 PostgreSQL 测试数据库 + 前置采购订单/产品/仓库数据。
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库 + 前置采购订单/产品/仓库数据"]
    async fn 测试_采购收货全流程_创建到确认() {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("需设置 TEST_DATABASE_URL 环境变量");
        let db = Database::connect(&db_url).await.expect("DB 连接失败");
        let svc = PurchaseReceiptService::new(Arc::new(db));

        // 1. 创建（DRAFT）
        let req = sample_request();
        let receipt = svc.create_receipt(req, 1).await.expect("创建失败");
        assert_eq!(receipt.receipt_status, purchase_receipt::DRAFT);

        // 2. 确认（DRAFT → CONFIRMED，触发库存更新 + 应付账单生成）
        let receipt = svc
            .confirm_receipt(receipt.id, 1)
            .await
            .expect("确认失败");
        assert_eq!(receipt.receipt_status, purchase_receipt::CONFIRMED);

        // 注：COMPLETED 状态当前在 PurchaseReceiptService 中无公开方法触发，
        // 由后续业务流程（如入库检验完成）自动标记，此处不覆盖。
    }
}
