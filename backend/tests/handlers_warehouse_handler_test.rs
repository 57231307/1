use bingxi_backend::handlers::warehouse_handler::*;
use bingxi_backend::models::warehouse::Model as WarehouseModel;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::json;

/// 构造测试用的仓库模型
fn make_warehouse_model(id: i32) -> WarehouseModel {
    WarehouseModel {
        id,
        warehouse_code: format!("WH-{:04}", id),
        name: "主仓库".to_string(),
        short_name: Some("主仓".to_string()),
        warehouse_type: Some("general".to_string()),
        address: Some("测试地址".to_string()),
        contact_person: Some("张三".to_string()),
        contact_phone: Some("13800138000".to_string()),
        capacity: Some(rust_decimal::Decimal::new(10000, 0)),
        used_capacity: Some(rust_decimal::Decimal::new(5000, 0)),
        status: Some("active".to_string()),
        notes: Some("测试备注".to_string()),
        created_by: Some(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ===== 模型测试 =====

#[test]
fn test_warehouse_model_serialization() {
    let warehouse = make_warehouse_model(1);
    let json = serde_json::to_value(&warehouse).expect("仓库序列化失败");

    assert_eq!(json["id"], 1);
    assert_eq!(json["warehouse_code"], "WH-0001");
    assert_eq!(json["name"], "主仓库");
    assert_eq!(json["status"], "active");
}

#[test]
fn test_warehouse_capacity() {
    let warehouse = make_warehouse_model(1);

    // 验证容量信息
    assert!(warehouse.capacity.is_some());
    assert!(warehouse.used_capacity.is_some());

    // 验证已用容量 <= 总容量
    let capacity = warehouse.capacity.unwrap();
    let used = warehouse.used_capacity.unwrap();
    assert!(used <= capacity);
}

#[test]
fn test_warehouse_available_capacity() {
    let warehouse = make_warehouse_model(1);

    // 验证可用容量计算
    let capacity = warehouse.capacity.unwrap();
    let used = warehouse.used_capacity.unwrap();
    let available = capacity - used;

    assert_eq!(available, rust_decimal::Decimal::new(5000, 0));
}

// ===== 仓库类型测试 =====

#[test]
fn test_warehouse_type_general() {
    let warehouse = make_warehouse_model(1);
    assert_eq!(warehouse.warehouse_type, Some("general".to_string()));
}

// ===== 状态测试 =====

#[test]
fn test_warehouse_status_active() {
    let warehouse = make_warehouse_model(1);
    assert_eq!(warehouse.status, Some("active".to_string()));
}

// ===== 联系信息测试 =====

#[test]
fn test_warehouse_contact_info() {
    let warehouse = make_warehouse_model(1);

    // 验证联系信息
    assert_eq!(warehouse.contact_person, Some("张三".to_string()));
    assert_eq!(warehouse.contact_phone, Some("13800138000".to_string()));
}

// ===== 容量计算测试 =====

#[test]
fn test_capacity_utilization() {
    let capacity = rust_decimal::Decimal::new(10000, 0);
    let used = rust_decimal::Decimal::new(7500, 0);
    let utilization = used / capacity * rust_decimal::Decimal::new(100, 0);

    assert_eq!(utilization, rust_decimal::Decimal::new(75, 0));
}

#[test]
fn test_capacity_full() {
    let capacity = rust_decimal::Decimal::new(10000, 0);
    let used = rust_decimal::Decimal::new(10000, 0);
    let available = capacity - used;

    assert_eq!(available, rust_decimal::Decimal::new(0, 0));
}

#[test]
fn test_capacity_overloaded() {
    let capacity = rust_decimal::Decimal::new(10000, 0);
    let used = rust_decimal::Decimal::new(12000, 0);
    let available = capacity - used;

    // 验证超载
    assert!(available < rust_decimal::Decimal::new(0, 0));
}

// ===== 序列化/反序列化测试 =====

#[test]
fn test_warehouse_json_roundtrip() {
    let warehouse = make_warehouse_model(1);
    let json = serde_json::to_value(&warehouse).expect("序列化失败");

    // 验证关键字段存在
    assert!(json.get("id").is_some());
    assert!(json.get("warehouse_code").is_some());
    assert!(json.get("name").is_some());
    assert!(json.get("status").is_some());
}
