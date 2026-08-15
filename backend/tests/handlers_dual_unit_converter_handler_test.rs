// 批次 351 v12 复审 P1-3：移除未使用的 rust_decimal::prelude::*（测试代码使用全路径）
// P9-1: 引入 decs! 宏替代 .from_str().unwrap()
// 批次 343 v11 复审 P3 修复：移除 #[allow(unused_imports)]，decs! 宏已被广泛使用
// decs 宏在测试中不可用，使用 Decimal::from_str 替代

use bingxi_backend::decs;
use bingxi_backend::handlers::dual_unit_converter_handler::ConvertUnitRequest;
use bingxi_backend::handlers::dual_unit_converter_handler::ValidateDualUnitRequest;
use bingxi_backend::ymd;

#[test]
fn test_convert_unit_request_deserialize() {
    let json = r#"
    {
        "value": "100.000",
        "from_unit": "meters",
        "gram_weight": "180.00",
        "width_cm": "180.00"
    }
    "#;

    let req: ConvertUnitRequest =
        serde_json::from_str(json).expect("P9-1: 测试夹具 JSON 反序列化失败");
    assert_eq!(req.value, decs!("100.000"));
    assert_eq!(req.from_unit, "meters");
    assert_eq!(req.gram_weight, decs!("180.00"));
    assert_eq!(req.width_cm, decs!("180.00"));
}

#[test]
fn test_validate_dual_unit_request_deserialize() {
    let json = r#"
    {
        "quantity_meters": "100.000",
        "quantity_kg": "3.240",
        "gram_weight": "180.00",
        "width_cm": "180.00",
        "tolerance": "0.005"
    }
    "#;

    let req: ValidateDualUnitRequest =
        serde_json::from_str(json).expect("P9-1: 测试夹具 JSON 反序列化失败");
    assert_eq!(req.quantity_meters, decs!("100.000"));
    assert_eq!(req.quantity_kg, decs!("3.240"));
    assert_eq!(req.gram_weight, decs!("180.00"));
    assert_eq!(req.width_cm, decs!("180.00"));
    assert_eq!(req.tolerance, Some(decs!("0.005")));
}
