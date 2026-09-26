//! 运单状态机与状态入参校验（handlers/logistics_handler.rs）
//!
//! 覆盖两条曾被绕过的路径：任意字符串都能写进 `logistics_waybills.status`，
//! 以及 `SIGNED` 可从通用更新接口写入从而跳过应收确认。

use bingxi_backend::handlers::logistics_handler::{
    is_legal_waybill_transition, validate_status_param,
};
use bingxi_backend::models::status::logistics_waybill as waybill_status;

#[test]
fn test_status_values_are_the_uppercase_state_machine_set() {
    // 库里实际写入的就是这三个大写值，前端与筛选入参必须与之完全一致
    assert_eq!(waybill_status::ALL, &["IN_TRANSIT", "DELIVERED", "SIGNED"]);
}

#[test]
fn test_validate_status_param_accepts_only_state_machine_values() {
    for value in waybill_status::ALL {
        assert_eq!(validate_status_param(value, "status").unwrap(), *value);
    }
    // 旧前端词表：小写与库里从不存在的取值必须被拒
    for bad in [
        "shipped",
        "in_transit",
        "arrived",
        "pickup",
        "cancelled",
        "",
    ] {
        let err = validate_status_param(bad, "status");
        assert!(err.is_err(), "非法状态取值被放行：{bad}");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("IN_TRANSIT") && msg.contains("DELIVERED") && msg.contains("SIGNED"),
            "错误信息未列出合法值，调用方无法自纠：{msg}"
        );
    }
}

#[test]
fn test_only_in_transit_to_delivered_is_allowed_via_update_endpoint() {
    assert!(is_legal_waybill_transition(
        waybill_status::IN_TRANSIT,
        waybill_status::DELIVERED
    ));

    // 签收必须走签收端点：通用更新口放行 SIGNED 等于绕过应收确认
    assert!(!is_legal_waybill_transition(
        waybill_status::IN_TRANSIT,
        waybill_status::SIGNED
    ));
    assert!(!is_legal_waybill_transition(
        waybill_status::DELIVERED,
        waybill_status::SIGNED
    ));
    // 无回退、无原地跳转
    assert!(!is_legal_waybill_transition(
        waybill_status::DELIVERED,
        waybill_status::IN_TRANSIT
    ));
    assert!(!is_legal_waybill_transition(
        waybill_status::SIGNED,
        waybill_status::DELIVERED
    ));
    assert!(!is_legal_waybill_transition(
        waybill_status::IN_TRANSIT,
        waybill_status::IN_TRANSIT
    ));
    // 脏数据状态（历史字面量）不得继续推进
    assert!(!is_legal_waybill_transition(
        "shipped",
        waybill_status::DELIVERED
    ));
    assert!(!is_legal_waybill_transition("", waybill_status::DELIVERED));
}
