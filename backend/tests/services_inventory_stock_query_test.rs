use bingxi_backend::models::inventory_stock;
use bingxi_backend::services::inventory_stock_query::compute_alert_type;
use bingxi_backend::services::stock_alert::{ALERT_TYPE_NORMAL, AlertType};
use chrono::{Duration, Utc};
use rust_decimal::Decimal;

/// 构造测试用库存 Model（默认"正常"状态）
fn make_stock_model() -> inventory_stock::Model {
    inventory_stock::Model {
        id: 1,
        warehouse_id: 1,
        product_id: 1,
        quantity_on_hand: Decimal::from(100),
        quantity_available: Decimal::from(100),
        quantity_reserved: Decimal::from(0),
        quantity_shipped: Decimal::from(0),
        quantity_incoming: Decimal::from(0),
        reorder_point: Decimal::from(0),
        max_stock_point: Decimal::from(0),
        reorder_quantity: Decimal::from(0),
        bin_location: None,
        last_count_date: None,
        last_movement_date: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        batch_no: "B001".to_string(),
        color_no: "C001".to_string(),
        dye_lot_no: Some("DL001".to_string()),
        grade: "一等品".to_string(),
        production_date: None,
        expiry_date: None,
        quantity_meters: Decimal::from(100),
        quantity_kg: Decimal::from(50),
    }
}

#[test]
fn test_normal_stock_alert() {
    let model = make_stock_model();
    let alert = compute_alert_type(&model);
    assert_eq!(alert, ALERT_TYPE_NORMAL);
}

#[test]
fn test_low_stock_alert() {
    let mut model = make_stock_model();
    model.quantity_on_hand = Decimal::from(5);
    model.reorder_point = Decimal::from(10);
    let alert = compute_alert_type(&model);
    assert_eq!(alert, AlertType::LowStock.code());
}

#[test]
fn test_overstock_alert() {
    let mut model = make_stock_model();
    model.quantity_on_hand = Decimal::from(150);
    model.max_stock_point = Decimal::from(100);
    let alert = compute_alert_type(&model);
    assert_eq!(alert, AlertType::OverStock.code());
}

#[test]
fn test_no_movement_alert() {
    let mut model = make_stock_model();
    model.last_movement_date = Some(Utc::now() - Duration::days(100));
    let alert = compute_alert_type(&model);
    assert_eq!(alert, AlertType::SlowMoving.code());
}

#[test]
fn test_expired_alert() {
    let mut model = make_stock_model();
    model.expiry_date = Some(Utc::now() - Duration::days(10));
    let alert = compute_alert_type(&model);
    assert_eq!(alert, AlertType::Expiring.code());
}
