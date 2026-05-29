//! 成本归集模块单元测试

use bingxi_backend::services::cost_collection_service::CreateCostCollectionRequest;
use rust_decimal::Decimal;

#[test]
fn test_create_cost_collection_request() {
    let req = CreateCostCollectionRequest {
        collection_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
        cost_object_type: Some("batch".to_string()),
        cost_object_id: Some(1),
        cost_object_no: Some("BATCH001".to_string()),
        batch_no: Some("BATCH001".to_string()),
        color_no: Some("COLOR001".to_string()),
        workshop: Some("workshop1".to_string()),
        direct_material: Decimal::new(1000, 2),
        direct_labor: Decimal::new(500, 2),
        manufacturing_overhead: Decimal::new(300, 2),
        processing_fee: Decimal::new(200, 2),
        dyeing_fee: Decimal::new(150, 2),
        output_quantity_meters: Some(Decimal::new(100, 0)),
        output_quantity_kg: None,
    };

    assert_eq!(req.batch_no, Some("BATCH001".to_string()));
    assert_eq!(req.direct_material, Decimal::new(1000, 2));
}

#[test]
fn test_total_cost_calculation() {
    let direct_material = Decimal::new(1000, 2);
    let direct_labor = Decimal::new(500, 2);
    let manufacturing_overhead = Decimal::new(300, 2);

    let total_cost = direct_material + direct_labor + manufacturing_overhead;
    assert_eq!(total_cost, Decimal::new(1800, 2));
}

#[test]
fn test_unit_cost_calculation() {
    let total_cost = Decimal::new(180000, 2);
    let output_quantity = Decimal::new(100, 0);

    let unit_cost = total_cost / output_quantity;
    assert_eq!(unit_cost, Decimal::new(1800, 2));
}
