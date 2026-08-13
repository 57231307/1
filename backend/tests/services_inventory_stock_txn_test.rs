// 引入 decs! 测试夹具宏，用于从字符串解析 Decimal
// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use rust_decimal::Decimal;

// ========== RecordTransactionArgs 构造测试 ==========

/// 验证 RecordTransactionArgs 入库场景下所有字段被正确设置
#[test]
fn test_record_transaction_args_rkcj_zdwz() {
    let args = RecordTransactionArgs {
        transaction_type: "IN".to_string(),
        product_id: 1001,
        warehouse_id: 2002,
        batch_no: "BATCH-2024-001".to_string(),
        color_no: "COLOR-RED".to_string(),
        dye_lot_no: Some("LOT-A1".to_string()),
        grade: "A".to_string(),
        quantity_meters: decs!("500.5"),
        quantity_kg: decs!("125.25"),
        source_bill_type: Some("PURCHASE".to_string()),
        source_bill_no: Some("PO-2024-0001".to_string()),
        source_bill_id: Some(9001),
        quantity_before_meters: Some(decs!("0")),
        quantity_before_kg: Some(decs!("0")),
        quantity_after_meters: Some(decs!("500.5")),
        quantity_after_kg: Some(decs!("125.25")),
        notes: Some("采购入库".to_string()),
        created_by: Some(1),
    };

    // 逐字段断言，验证入库场景的完整构造
    assert_eq!(args.transaction_type, "IN");
    assert_eq!(args.product_id, 1001);
    assert_eq!(args.warehouse_id, 2002);
    assert_eq!(args.batch_no, "BATCH-2024-001");
    assert_eq!(args.color_no, "COLOR-RED");
    assert_eq!(args.dye_lot_no.as_deref(), Some("LOT-A1"));
    assert_eq!(args.grade, "A");
    assert_eq!(args.quantity_meters, decs!("500.5"));
    assert_eq!(args.quantity_kg, decs!("125.25"));
    assert_eq!(args.source_bill_type.as_deref(), Some("PURCHASE"));
    assert_eq!(args.source_bill_no.as_deref(), Some("PO-2024-0001"));
    assert_eq!(args.source_bill_id, Some(9001));
    // 入库前为 0，入库后等于本次入库量
    assert_eq!(args.quantity_before_meters, Some(decs!("0")));
    assert_eq!(args.quantity_before_kg, Some(decs!("0")));
    assert_eq!(args.quantity_after_meters, Some(decs!("500.5")));
    assert_eq!(args.quantity_after_kg, Some(decs!("125.25")));
    assert_eq!(args.notes.as_deref(), Some("采购入库"));
    assert_eq!(args.created_by, Some(1));
}

/// 验证 RecordTransactionArgs 出库场景下所有字段被正确设置
#[test]
fn test_record_transaction_args_ckcj_zdwz() {
    let args = RecordTransactionArgs {
        transaction_type: "OUT".to_string(),
        product_id: 3003,
        warehouse_id: 4004,
        batch_no: "BATCH-2024-002".to_string(),
        color_no: "COLOR-BLUE".to_string(),
        dye_lot_no: None,
        grade: "B".to_string(),
        quantity_meters: decs!("100.0"),
        quantity_kg: decs!("25.0"),
        source_bill_type: Some("SALES".to_string()),
        source_bill_no: Some("SO-2024-0002".to_string()),
        source_bill_id: Some(8002),
        quantity_before_meters: Some(decs!("500.0")),
        quantity_before_kg: Some(decs!("125.0")),
        quantity_after_meters: Some(decs!("400.0")),
        quantity_after_kg: Some(decs!("100.0")),
        notes: None,
        created_by: Some(2),
    };

    // 逐字段断言，验证出库场景的完整构造
    assert_eq!(args.transaction_type, "OUT");
    assert_eq!(args.product_id, 3003);
    assert_eq!(args.warehouse_id, 4004);
    assert_eq!(args.batch_no, "BATCH-2024-002");
    assert_eq!(args.color_no, "COLOR-BLUE");
    // 出库场景下 dye_lot_no 可为 None
    assert_eq!(args.dye_lot_no, None);
    assert_eq!(args.grade, "B");
    assert_eq!(args.quantity_meters, decs!("100.0"));
    assert_eq!(args.quantity_kg, decs!("25.0"));
    assert_eq!(args.source_bill_type.as_deref(), Some("SALES"));
    assert_eq!(args.source_bill_no.as_deref(), Some("SO-2024-0002"));
    assert_eq!(args.source_bill_id, Some(8002));
    // 出库前为 500，出库 100 后剩余 400
    assert_eq!(args.quantity_before_meters, Some(decs!("500.0")));
    assert_eq!(args.quantity_before_kg, Some(decs!("125.0")));
    assert_eq!(args.quantity_after_meters, Some(decs!("400.0")));
    assert_eq!(args.quantity_after_kg, Some(decs!("100.0")));
    // 出库场景下 notes 可为 None
    assert_eq!(args.notes, None);
    assert_eq!(args.created_by, Some(2));
}

// ========== CreateStockFabricArgs 构造测试 ==========

/// 验证 CreateStockFabricArgs 含缸号场景下所有字段被正确设置
#[test]
fn test_create_stock_fabric_args_hgh_zdwz() {
    let args = CreateStockFabricArgs {
        warehouse_id: 1001,
        product_id: 2002,
        batch_no: "BATCH-2024-A001".to_string(),
        color_no: "RED-001".to_string(),
        dye_lot_no: Some("LOT-D01".to_string()),
        grade: "A".to_string(),
        quantity_meters: decs!("1000.0"),
        quantity_kg: decs!("250.0"),
        gram_weight: Some(decs!("250.0")),
        width: Some(decs!("150.0")),
        location_id: Some(5001),
        shelf_no: Some("A-01".to_string()),
        layer_no: Some("L1".to_string()),
    };

    // 逐字段断言，验证含缸号场景的完整构造
    assert_eq!(args.warehouse_id, 1001);
    assert_eq!(args.product_id, 2002);
    assert_eq!(args.batch_no, "BATCH-2024-A001");
    assert_eq!(args.color_no, "RED-001");
    assert_eq!(args.dye_lot_no.as_deref(), Some("LOT-D01"));
    assert_eq!(args.grade, "A");
    assert_eq!(args.quantity_meters, decs!("1000.0"));
    assert_eq!(args.quantity_kg, decs!("250.0"));
    assert_eq!(args.gram_weight, Some(decs!("250.0")));
    assert_eq!(args.width, Some(decs!("150.0")));
    assert_eq!(args.location_id, Some(5001));
    assert_eq!(args.shelf_no.as_deref(), Some("A-01"));
    assert_eq!(args.layer_no.as_deref(), Some("L1"));
}

/// 验证 CreateStockFabricArgs 不含缸号场景下所有可选字段为 None
#[test]
fn test_create_stock_fabric_args_bhgh_kxzdw_none() {
    let args = CreateStockFabricArgs {
        warehouse_id: 3003,
        product_id: 4004,
        batch_no: "BATCH-2024-B002".to_string(),
        color_no: "BLUE-002".to_string(),
        dye_lot_no: None,
        grade: "B".to_string(),
        quantity_meters: decs!("500.0"),
        quantity_kg: decs!("125.0"),
        gram_weight: None,
        width: None,
        location_id: None,
        shelf_no: None,
        layer_no: None,
    };

    // 逐字段断言，验证不含缸号场景的可选字段均为 None
    assert_eq!(args.warehouse_id, 3003);
    assert_eq!(args.product_id, 4004);
    assert_eq!(args.batch_no, "BATCH-2024-B002");
    assert_eq!(args.color_no, "BLUE-002");
    // 不含缸号场景下 dye_lot_no 为 None
    assert_eq!(args.dye_lot_no, None);
    assert_eq!(args.grade, "B");
    assert_eq!(args.quantity_meters, decs!("500.0"));
    assert_eq!(args.quantity_kg, decs!("125.0"));
    // 其他可选字段也应为 None
    assert_eq!(args.gram_weight, None);
    assert_eq!(args.width, None);
    assert_eq!(args.location_id, None);
    assert_eq!(args.shelf_no, None);
    assert_eq!(args.layer_no, None);
}

// ========== BusinessEvent::InventoryTransactionCreated 变体存在性验证 ==========

/// 验证 BusinessEvent::InventoryTransactionCreated 变体可被 match 匹配（此测试通过穷举 match 确认枚举变体存在，避免重构时该变体被误删导致编译错误延后暴露。）
#[test]
fn test_business_event_inventory_transaction_created_btcz() {
    let event = BusinessEvent::InventoryTransactionCreated {
        transaction_id: 1,
        transaction_type: "IN".to_string(),
        product_id: 100,
        warehouse_id: 200,
        quantity_meters: decs!("100.0"),
        quantity_kg: decs!("25.0"),
        source_bill_type: Some("PURCHASE".to_string()),
        source_bill_no: Some("PO-001".to_string()),
        source_bill_id: Some(1),
        batch_no: "BATCH-001".to_string(),
        color_no: "RED-001".to_string(),
        created_by: Some(1),
    };

    // 通过 match 确认 InventoryTransactionCreated 变体可被匹配
    let matched = match event {
        BusinessEvent::InventoryTransactionCreated {
            transaction_id,
            transaction_type,
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } => {
            // 逐一验证字段被正确读取
            assert_eq!(transaction_id, 1);
            assert_eq!(transaction_type, "IN");
            assert_eq!(product_id, 100);
            assert_eq!(warehouse_id, 200);
            assert_eq!(quantity_meters, decs!("100.0"));
            assert_eq!(quantity_kg, decs!("25.0"));
            assert_eq!(source_bill_type.as_deref(), Some("PURCHASE"));
            assert_eq!(source_bill_no.as_deref(), Some("PO-001"));
            assert_eq!(source_bill_id, Some(1));
            assert_eq!(batch_no, "BATCH-001");
            assert_eq!(color_no, "RED-001");
            assert_eq!(created_by, Some(1));
            true
        }
        _ => false,
    };

    // 确认进入了正确变体的分支
    assert!(
        matched,
        "BusinessEvent 应匹配到 InventoryTransactionCreated 变体"
    );
}
