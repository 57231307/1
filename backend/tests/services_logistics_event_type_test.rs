//! 物流轨迹事件类型词表（models/status/bpm_crm_contract.rs 的 logistics_event_type）
//!
//! 该列此前无取值校验，而运单主状态用的是另一套大写码，两者极易被混写。
//! 本用例钉住事件词表的小写稳定码，并确认它与运单状态词表不互相包含，
//! 防止有人把 `IN_TRANSIT`/`DELIVERED` 当成事件类型写进轨迹。

use bingxi_backend::models::status::bpm_crm_contract::logistics_event_type;
use bingxi_backend::models::status::bpm_crm_contract::logistics_waybill;

#[test]
fn test_event_type_values_are_the_lowercase_codes() {
    assert_eq!(
        logistics_event_type::ALL,
        &["pickup", "in_transit", "arrived", "delivered"]
    );
}

#[test]
fn test_event_and_waybill_status_domains_do_not_overlap() {
    for event in logistics_event_type::ALL {
        assert!(
            !logistics_waybill::ALL.contains(event),
            "事件类型与运单状态重名，两套词表会被混写：{event}"
        );
    }
}
