use bingxi_backend::models::color_card::card_status;
use bingxi_backend::models::status::wage_energy_chemical_business::*;
use bingxi_backend::services::color_card_service::ColorCardCrudService;
use bingxi_backend::services::wage_energy_chemical_business;

/// V15 P2 B05-P2-4：合法流转 draft → issued → received → used → expired
#[test]
fn test_valid_forward_transition_chain() {
    assert!(
        ColorCardCrudService::validate_color_card_status_transition("draft", card_status::ISSUED)
            .is_ok()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition("active", card_status::ISSUED)
            .is_ok()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::ISSUED,
            card_status::RECEIVED
        )
        .is_ok()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::RECEIVED,
            card_status::USED
        )
        .is_ok()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::USED,
            card_status::EXPIRED
        )
        .is_ok()
    );
}

/// V15 P2 B05-P2-4：任意非终态 → lost/archived 合法
#[test]
fn test_terminal_transitions_from_any_non_terminal() {
    for from in ["draft", "active", "issued", "received", "used"] {
        assert!(
            ColorCardCrudService::validate_color_card_status_transition(from, card_status::LOST)
                .is_ok()
        );
        assert!(
            ColorCardCrudService::validate_color_card_status_transition(
                from,
                card_status::ARCHIVED
            )
            .is_ok()
        );
    }
}

/// V15 P2 B05-P2-4：终态不可再流转
#[test]
fn test_terminal_state_no_outgoing() {
    for terminal in [
        card_status::EXPIRED,
        card_status::LOST,
        card_status::ARCHIVED,
    ] {
        assert!(
            ColorCardCrudService::validate_color_card_status_transition(
                terminal,
                card_status::ISSUED
            )
            .is_err()
        );
        assert!(
            ColorCardCrudService::validate_color_card_status_transition(
                terminal,
                card_status::RECEIVED
            )
            .is_err()
        );
    }
}

/// V15 P2 B05-P2-4：非法跳转（如 draft → received / issued → used）
#[test]
fn test_invalid_skip_transitions() {
    assert!(
        ColorCardCrudService::validate_color_card_status_transition("draft", card_status::RECEIVED)
            .is_err()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::ISSUED,
            card_status::USED
        )
        .is_err()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::RECEIVED,
            card_status::EXPIRED
        )
        .is_err()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition("draft", card_status::USED)
            .is_err()
    );
}

/// V15 P2 B05-P2-4：非法回退（如 issued → draft / used → received）
#[test]
fn test_invalid_backward_transitions() {
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(card_status::ISSUED, "draft")
            .is_err()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::USED,
            card_status::RECEIVED
        )
        .is_err()
    );
    assert!(
        ColorCardCrudService::validate_color_card_status_transition(
            card_status::RECEIVED,
            card_status::ISSUED
        )
        .is_err()
    );
}
