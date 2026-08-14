use bingxi_backend::services::dye_batch_state_machine_validation::*;

#[test]
fn test_status_transition() {
    // V15 P1 batch-19：opened → accepted → processing → resolved → evaluated → closed
    assert!(is_valid_transition(Some("opened"), "accepted", "open_to_accept"));
    assert!(is_valid_transition(Some("accepted"), "processing", "accept_to_process"));
    assert!(is_valid_transition(Some("processing"), "resolved", "process_to_resolve"));
    assert!(is_valid_transition(Some("resolved"), "evaluated", "resolve_to_evaluate"));
    assert!(is_valid_transition(Some("evaluated"), "closed", "evaluate_to_close"));
    assert!(!is_valid_transition(Some("closed"), "processing", "closed_to_process"));
    // opened 不能直接跳到 processing（需先 accepted）
    assert!(!is_valid_transition(Some("opened"), "processing", "open_to_process"));
    // opened 不能直接跳到 resolved
    assert!(!is_valid_transition(Some("opened"), "resolved", "open_to_resolve"));
}
