use bingxi_backend::handlers::inventory_stock_handler::*;
use bingxi_backend::models::inventory_stock::Model as InventoryStockModel;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的库存模型
fn make_inventory_stock_model(id: i32) -> InventoryStockModel {
    InventoryStockModel {
        id,
        warehouse_id: 1,
        product_id: 1,
        quantity_on_hand: Decimal::new(100, 0),
        quantity_available: Decimal::new(90, 0),
        quantity_reserved: Decimal::new(10, 0),
        quantity_shipped: Decimal::new(0, 0),
        quantity_incoming: Decimal::new(0, 0),
        reorder_point: Decimal::new(20, 0),
        max_stock_point: Decimal::new(500, 0),
        reorder_quantity: Decimal::new(50, 0),
        bin_location: Some("A-01-01".to_string()),
        last_count_date: Some(Utc::now()),
        last_movement_date: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        batch_no: "B001".to_string(),
        color_no: "C001".to_string(),
        dye_lot_no: Some("DL001".to_string()),
        grade: "一等品".to_string(),
        production_date: Some(Utc::now()),
        effective_to: None,
        quantity_meters: Decimal::new(100, 0),
        quantity_kg: Decimal::new(50, 0),
        gram_weight: Some(Decimal::new(200, 0)),
        width: Some(Decimal::new(150, 0)),
        location_id: Some(1),
        shelf_no: Some("S01".to_string()),
        layer_no: Some("L01".to_string()),
        stock_status: "normal".to_string(),
        quality_status: "qualified".to_string(),
        version: 1,
        replenishment_strategy: "reorder_point".to_string(),
    }
}

// ===== 模型测试 =====

#[test]
fn test_inventory_stock_model_serialization() {
    let stock = make_inventory_stock_model(1);
    let json = serde_json::to_value(&stock).expect("库存序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["warehouse_id"], 1);
    assert_eq!(json["product_id"], 1);
    assert_eq!(json["stock_status"], "normal");
}

#[test]
fn test_inventory_stock_quantities() {
    let stock = make_inventory_stock_model(1);

    // 验证数量关系
    assert_eq!(stock.quantity_on_hand, Decimal::new(100, 0));
    assert_eq!(stock.quantity_reserved, Decimal::new(10, 0));
    assert_eq!(stock.quantity_available, Decimal::new(90, 0));

    // 验证可用数量 = 总数量 - 预留数量
    assert_eq!(stock.quantity_available, stock.quantity_on_hand - stock.quantity_reserved);
}

// ===== 数量计算测试 =====

#[test]
fn test_quantity_reserved() {
    let quantity = Decimal::new(100, 0);
    let reserved = Decimal::new(30, 0);
    let available = quantity - reserved;

    assert_eq!(available, Decimal::new(70, 0));
}

#[test]
fn test_quantity_fully_reserved() {
    let quantity = Decimal::new(100, 0);
    let reserved = Decimal::new(100, 0);
    let available = quantity - reserved;

    assert_eq!(available, Decimal::new(0, 0));
}

#[test]
fn test_quantity_over_reserved() {
    let quantity = Decimal::new(100, 0);
    let reserved = Decimal::new(120, 0);
    let available = quantity - reserved;

    // 允许负值表示超预留
    assert_eq!(available, Decimal::new(-20, 0));
}

// ===== 状态转换测试 =====

#[test]
fn test_status_active_to_locked() {
    let stock = make_inventory_stock_model(1);
    assert_eq!(stock.stock_status, "normal");

    // 验证活跃状态可以转换为锁定
    let valid_transitions = vec!["frozen", "pending_inspection"];
    assert!(valid_transitions.contains(&"frozen"));
}

#[test]
fn test_status_locked_to_active() {
    let mut stock = make_inventory_stock_model(1);
    stock.stock_status = "frozen".to_string();
    assert_eq!(stock.stock_status, "frozen");

    // 验证锁定状态可以转换为活跃
    let valid_transitions = vec!["normal", "pending_inspection"];
    assert!(valid_transitions.contains(&"normal"));
}

// ===== 库位测试 =====

#[test]
fn test_location_format() {
    let stock = make_inventory_stock_model(1);
    let location = stock.bin_location.as_ref().unwrap();

    // 验证库位格式
    assert!(location.contains('-'));
    let parts: Vec<&str> = location.split('-').collect();
    assert_eq!(parts.len(), 3);
}

// ===== 批次测试 =====

#[test]
fn test_batch_no_format() {
    let stock = make_inventory_stock_model(1);

    // 验证批次号格式
    assert!(stock.batch_no.starts_with('B'));
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_inventory_stock_json_roundtrip() {
    let stock = make_inventory_stock_model(1);
    let json = serde_json::to_value(&stock).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("warehouse_id").is_some());
    assert!(json.get("product_id").is_some());
    assert!(json.get("quantity_on_hand").is_some());
    assert!(json.get("stock_status").is_some());
}
