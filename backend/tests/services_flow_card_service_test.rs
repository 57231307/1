use bingxi_backend::models::status::flow_card as card_status;
use bingxi_backend::services::flow_card_service::FlowCardService;

/// 测试流转卡状态流转校验
#[test]
fn test_validate_status_transition_normal() {
    // 正常流转路径
    assert!(
        FlowCardService::validate_status_transition(card_status::PENDING, card_status::SCHEDULED)
            .is_ok()
    );
    assert!(
        FlowCardService::validate_status_transition(card_status::SCHEDULED, card_status::PREPARING)
            .is_ok()
    );
    assert!(
        FlowCardService::validate_status_transition(card_status::PREPARING, card_status::DYEING)
            .is_ok()
    );
    assert!(
        FlowCardService::validate_status_transition(card_status::DYEING, card_status::DYED).is_ok()
    );
    assert!(
        FlowCardService::validate_status_transition(card_status::DYED, card_status::INSPECTING)
            .is_ok()
    );
    assert!(
        FlowCardService::validate_status_transition(
            card_status::INSPECTING,
            card_status::COMPLETED
        )
        .is_ok()
    );
    assert!(
        FlowCardService::validate_status_transition(card_status::COMPLETED, card_status::SHIPPED)
            .is_ok()
    );
}

/// 测试流转卡状态流转校验：非法路径
#[test]
fn test_validate_status_transition_illegal() {
    // pending 不能直接到 dyeing
    assert!(
        FlowCardService::validate_status_transition(card_status::PENDING, card_status::DYEING)
            .is_err()
    );
    // shipped 是终态，不可再流转
    assert!(
        FlowCardService::validate_status_transition(card_status::SHIPPED, card_status::PENDING)
            .is_err()
    );
    // terminated 只能回到 pending
    assert!(
        FlowCardService::validate_status_transition(
            card_status::TERMINATED,
            card_status::SCHEDULED
        )
        .is_err()
    );
    assert!(
        FlowCardService::validate_status_transition(card_status::TERMINATED, card_status::PENDING)
            .is_ok()
    );
}

/// 测试可更新状态校验
#[test]
fn test_validate_can_update() {
    assert!(FlowCardService::validate_can_update(card_status::PENDING).is_ok());
    assert!(FlowCardService::validate_can_update(card_status::SCHEDULED).is_ok());
    assert!(FlowCardService::validate_can_update(card_status::DYEING).is_err());
    assert!(FlowCardService::validate_can_update(card_status::COMPLETED).is_err());
}

/// 测试回修场景：INSPECTING 可回到 DYEING（回修订单重新进缸）
#[test]
fn test_validate_status_transition_rework() {
    // 验布发现质量问题需要回修染色
    assert!(
        FlowCardService::validate_status_transition(card_status::INSPECTING, card_status::DYEING)
            .is_ok()
    );
}
