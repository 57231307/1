use rust_decimal::Decimal;
use bingxi_backend::models::custom_order_create_dto::*;
use sea_orm::entity::prelude::Set;

/// 辅助函数：构造最小有效 CreateCustomOrderDto，便于测试 notes 字段透传
fn make_test_dto(notes: Option<String>) -> CreateCustomOrderDto {
    CreateCustomOrderDto {
        customer_id: 1,
        product_id: 1,
        color_id: None,
        spec: "测试规格".to_string(),
        quantity: Decimal::from(100),
        unit: "m".to_string(),
        custom_requirements: None,
        yarn_spec: None,
        dye_method: None,
        finishing_method: None,
        expected_delivery_date: None,
        sales_order_id: None,
        total_amount: None,
        currency: None,
        notes,
    }
}

/// 测试 CreateCustomOrderDto 的 notes 字段类型为 Option<String> 且透传正确
/// DTO 字段透传验证，完整 create_draft 流程需集成测试；（create_draft line 96 `notes: Set(dto.notes)` 将 DTO notes 持久化到 custom_orders.notes）
#[test]
fn test_notes_field_in_create_dto() {
    let dto = make_test_dto(Some("客户备注内容".to_string()));
    // 显式标注 Option<String> 类型，编译期验证 notes 字段类型正确
    let notes: Option<String> = dto.notes;
    assert_eq!(notes, Some("客户备注内容".to_string()));
}

/// 测试 notes=None 时 DTO 字段为 None，create_draft 会将 None 写入 custom_orders.notes（DTO 字段透传验证，完整 create_draft 流程需集成测试）
#[test]
fn test_notes_default_when_none() {
    let dto = make_test_dto(None);
    assert_eq!(dto.notes, None);
}