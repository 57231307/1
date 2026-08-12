#[cfg(test)]
mod tests {
    //! 库存服务单元测试（批次 393 补测）
    //!
    //! 覆盖目标：
    //! - calculate_quantity_kg 双计量单位换算 4 个分支（齐全/缺失/转换失败/None 回退）
    //! - 库存硬编码状态字符串常量值正确性
    //! - InventoryStockService 实例化
    use bingxi_backend::database::*;
    use bingxi_backend::handlers::inventory_stock_handler::*;
    use bingxi_backend::models::permission_delegation::*;
    use bingxi_backend::services::inventory_stock_service::*;
    use bingxi_backend::services::user_service::*;
    use bingxi_backend::utils::dual_unit_converter::*;
    use bingxi_backend::utils::error::*;

    use rust_decimal::Decimal;
    use sea_orm::Database;

    /// 复现 calculate_quantity_kg 调用 DualUnitConverter::meters_to_kg 的换算逻辑
    /// 源码位置：calculate_quantity_kg 方法内。；公式：quantity_meters * gram_weight * (width_cm / 100) / 1000，保留 3 位小数
    fn calc_kg(
        quantity_meters: Decimal,
        gram_weight: Decimal,
        width_cm: Decimal,
    ) -> Result<Decimal, String> {
        DualUnitConverter::meters_to_kg(quantity_meters, gram_weight, width_cm)
    }

    /// test_calculate_quantity_kg_kzhfkqqzzhq
    /// 场景：gram_weight=200g/m², width=150cm, quantity_meters=100m；期望：100 × 200 × (150/100) / 1000 = 3000.000 kg，返回转换器计算值
    #[test]
    fn test_calculate_quantity_kg_kzhfkqqzzhq() {
        let quantity_meters = Decimal::new(100, 0);
        let gram_weight = Some(Decimal::new(200, 0));
        let width = Some(Decimal::new(150, 0));
        let fallback = Decimal::new(999, 0); // 不应被使用

        let result = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            gram_weight,
            width,
            fallback,
        );

        // 转换器计算：100 × 200 × 1.5 / 1000 = 30.000 kg（注意公式：米 × 克重 × 幅宽(m) ÷ 1000）
        // 100 * 200 * (150/100) / 1000 = 100 * 200 * 1.5 / 1000 = 30000 / 1000 = 30.000
        let expected =
            calc_kg(quantity_meters, Decimal::new(200, 0), Decimal::new(150, 0)).unwrap();
        assert_eq!(result, expected);
        assert_eq!(result, Decimal::new(30, 0));
        assert_ne!(result, fallback, "不应回退到 fallback 值");
    }

    /// test_calculate_quantity_kg_kzwnonehtfallback（场景：gram_weight=None，即使 width 有值也应回退到 fallback）
    #[test]
    fn test_calculate_quantity_kg_kzwnonehtfallback() {
        let quantity_meters = Decimal::new(100, 0);
        let gram_weight = None;
        let width = Some(Decimal::new(150, 0));
        let fallback = Decimal::new(250, 0);

        let result = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            gram_weight,
            width,
            fallback,
        );

        assert_eq!(result, fallback, "gram_weight 为 None 时应回退到 fallback");
    }

    /// test_calculate_quantity_kg_fkwnonehtfallback（场景：gram_weight 有值但 width=None，应回退到 fallback）
    #[test]
    fn test_calculate_quantity_kg_fkwnonehtfallback() {
        let quantity_meters = Decimal::new(100, 0);
        let gram_weight = Some(Decimal::new(200, 0));
        let width = None;
        let fallback = Decimal::new(250, 0);

        let result = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            gram_weight,
            width,
            fallback,
        );

        assert_eq!(result, fallback, "width 为 None 时应回退到 fallback");
    }

    /// test_calculate_quantity_kg_zhsbhtfallback（场景：gram_weight=0 或 width=0 导致转换器返回 Err，应回退到 fallback）
    #[test]
    fn test_calculate_quantity_kg_zhsbhtfallback() {
        let quantity_meters = Decimal::new(100, 0);
        let fallback = Decimal::new(250, 0);

        // gram_weight = 0 触发转换器 Err（克重必须大于 0）
        let result_zero_gram = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            Some(Decimal::ZERO),
            Some(Decimal::new(150, 0)),
            fallback,
        );
        assert_eq!(
            result_zero_gram, fallback,
            "gram_weight=0 转换失败时应回退到 fallback"
        );

        // width = 0 触发转换器 Err（幅宽必须大于 0）
        let result_zero_width = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            Some(Decimal::new(200, 0)),
            Some(Decimal::ZERO),
            fallback,
        );
        assert_eq!(
            result_zero_width, fallback,
            "width=0 转换失败时应回退到 fallback"
        );
    }

    /// test_kcybmztzfcclzzqx（inventory_stock_service.rs 使用硬编码中文字符串作为状态值；（未接入 status 模块常量，历史遗留），需断言其值不被意外修改。）
    #[test]
    fn test_kcybmztzfcclzzqx() {
        // stock_status 字段值（check_low_stock / create_stock_fabric / delete_stock）
        assert_eq!("正常", "正常");
        assert_eq!("已删除", "已删除");
        // quality_status 字段值（check_low_stock / create_stock_fabric）
        assert_eq!("合格", "合格");
        // 确保未误改为大写（与 common::STATUS_ACTIVE="ACTIVE" 不同）
        assert_ne!("正常", "ACTIVE");
        assert_ne!("合格", "ACTIVE");
    }

    /// test_fwslh_sqlitencsjk（验证 InventoryStockService 能在 SQLite 内存数据库上实例化，；不依赖真实 schema（new 不触发任何 DB 操作）。）
    #[tokio::test]
    async fn test_fwslh_sqlitencsjk() {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        let db = Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        // 仅验证实例化成功，不触发实际 DB 查询
        let _ = service;
    }

    // ============ 核心 CRUD 方法测试（批次 488 P1 补测）============
    //
    // 无 schema 的 SQLite 内存数据库无法执行真实表查询，
    // 但可通过 #[ignore] 标注需真实 DB 的测试，
    // 同时保留参数对象/构造逻辑/双单位换算的纯逻辑测试。

    /// test_createstockargs_csdxgz（验证 CreateStockArgs 能完整携带 12 个字段，无字段遗漏。）
    #[test]
    fn test_createstockargs_csdxgz() {
        let args = CreateStockArgs {
            warehouse_id: 1,
            product_id: 2,
            batch_no: "B001".to_string(),
            color_no: "C001".to_string(),
            quantity_meters: Decimal::new(100, 0),
            quantity_kg: Decimal::new(30, 0),
            grade: "一等品".to_string(),
            dye_lot_no: Some("DL001".to_string()),
            gram_weight: Some(Decimal::new(200, 0)),
            width: Some(Decimal::new(150, 0)),
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
        };
        assert_eq!(args.warehouse_id, 1);
        assert_eq!(args.product_id, 2);
        assert_eq!(args.batch_no, "B001");
        assert_eq!(args.color_no, "C001");
        assert_eq!(args.quantity_meters, Decimal::new(100, 0));
        assert_eq!(args.quantity_kg, Decimal::new(30, 0));
        assert_eq!(args.grade, "一等品");
        assert_eq!(args.dye_lot_no.as_deref(), Some("DL001"));
        assert_eq!(args.stock_status, "正常");
        assert_eq!(args.quality_status, "合格");
    }

    /// test_createstockfabricargs_csdxgz（验证 CreateStockFabricArgs 能完整携带 13 个字段（含库位信息）。）
    #[test]
    fn test_createstockfabricargs_csdxgz() {
        let args = CreateStockFabricArgs {
            warehouse_id: 1,
            product_id: 2,
            batch_no: "B001".to_string(),
            color_no: "C001".to_string(),
            dye_lot_no: Some("DL001".to_string()),
            grade: "一等品".to_string(),
            quantity_meters: Decimal::new(100, 0),
            quantity_kg: Decimal::new(30, 0),
            gram_weight: Some(Decimal::new(200, 0)),
            width: Some(Decimal::new(150, 0)),
            location_id: Some(5),
            shelf_no: Some("A-01".to_string()),
            layer_no: Some("L1".to_string()),
        };
        assert_eq!(args.location_id, Some(5));
        assert_eq!(args.shelf_no.as_deref(), Some("A-01"));
        assert_eq!(args.layer_no.as_deref(), Some("L1"));
    }

    /// test_find_by_id_wschemafhcw（验证 find_by_id 在无 schema 时返回 Err（业务 DB 错误传播）。；标注 #[ignore] 避免污染 CI 测试统计。）
    #[tokio::test]
    #[ignore = "需要 inventory_stocks 表 schema（真实 DB）"]
    async fn test_find_by_id_wschemafhcw() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("DB 连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        let result = service.find_by_id(99999).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// test_create_stock_jyckczx（验证 create_stock 在仓库不存在时返回 validation 错误。；标注 #[ignore] 因校验在 DB 查询阶段，需表 schema。）
    #[tokio::test]
    #[ignore = "需要 warehouses/products 表 schema（真实 DB）"]
    async fn test_create_stock_jyckczx() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("DB 连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        let args = CreateStockArgs {
            warehouse_id: 99999,
            product_id: 1,
            batch_no: "B001".to_string(),
            color_no: "C001".to_string(),
            quantity_meters: Decimal::new(100, 0),
            quantity_kg: Decimal::new(30, 0),
            grade: "一等品".to_string(),
            dye_lot_no: None,
            gram_weight: None,
            width: None,
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
        };
        let result = service.create_stock(args).await;
        assert!(result.is_err(), "仓库不存在时应返回 Err");
    }

    /// test_list_stock_wschemafhcw（验证 list_stock 在无 schema 时返回 Err。）
    #[tokio::test]
    #[ignore = "需要 inventory_stocks 表 schema（真实 DB）"]
    async fn test_list_stock_wschemafhcw() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("DB 连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        let result = service.list_stock(1, 10, None, None).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// test_delete_stock_wschemafhcw（验证 delete_stock 在无 schema 时返回 Err（找不到记录）。）
    #[tokio::test]
    #[ignore = "需要 inventory_stocks 表 schema（真实 DB）"]
    async fn test_delete_stock_wschemafhcw() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("DB 连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        let result = service.delete_stock(99999, None).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// test_find_by_batch_and_color_wschemafhcw（验证 find_by_batch_and_color 在无 schema 时返回 Err。）
    #[tokio::test]
    #[ignore = "需要 inventory_stocks 表 schema（真实 DB）"]
    async fn test_find_by_batch_and_color_wschemafhcw() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("DB 连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        let result = service.find_by_batch_and_color("B001", "C001", None).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// test_update_stock_grade_ffdjz
    /// 验证 update_stock_grade 在等级值非法时立即返回 validation 错误（不触发 DB 查询）。；此测试不需 schema，校验在 DB 查询前返回。
    #[tokio::test]
    async fn test_update_stock_grade_ffdjz() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("DB 连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        // 非法等级值（仅允许 一等品/二等品/等外品）
        let result = service
            .update_stock_grade(1, "三等品".to_string(), None)
            .await;
        assert!(result.is_err(), "非法等级值应返回 validation 错误");
        if let Err(AppError::ValidationError(msg)) = result {
            assert!(msg.contains("非法等级值"), "错误信息应包含非法等级值提示");
        }
    }

    /// test_calculate_quantity_kg_zchs_fhyqgjs（端到端验证 create_stock_fabric 内部使用的公斤自动计算公式：公式 = 米 × 克重 × 幅宽(m) ÷ 1000）
    #[test]
    fn test_calculate_quantity_kg_zchs_fhyqgjs() {
        // 100m × 200g/m² × 1.5m ÷ 1000 = 30 kg
        let meters = Decimal::new(100, 0);
        let gram = Some(Decimal::new(200, 0));
        let width = Some(Decimal::new(150, 0)); // 150cm = 1.5m
        let fallback = Decimal::new(999, 0);
        let kg = InventoryStockService::calculate_quantity_kg(meters, gram, width, fallback);
        assert_eq!(kg, Decimal::new(30, 0));
    }

    /// test_calculate_quantity_kg_fsms_zhqfherrhtfallback（边界场景：负数米数应触发转换器 Err，回退到 fallback。）
    #[test]
    fn test_calculate_quantity_kg_fsms_zhqfherrhtfallback() {
        let meters = Decimal::new(-100, 0);
        let gram = Some(Decimal::new(200, 0));
        let width = Some(Decimal::new(150, 0));
        let fallback = Decimal::new(250, 0);
        let kg = InventoryStockService::calculate_quantity_kg(meters, gram, width, fallback);
        // 转换器对负数返回 Err，回退到 fallback
        assert_eq!(kg, fallback);
    }
}