use bingxi_backend::services::dye_batch_state_machine_validation::*;

#[test]
fn test_status_transition() {
    // V15 P1 batch-19：opened → accepted → processing → resolved → evaluated → closed
    assert!(is_valid_transition("opened", "accepted"));
    assert!(is_valid_transition("accepted", "processing"));
    assert!(is_valid_transition("processing", "resolved"));
    assert!(is_valid_transition("resolved", "evaluated"));
    assert!(is_valid_transition("evaluated", "closed"));
    assert!(!is_valid_transition("closed", "processing"));
    // opened 不能直接跳到 processing（需先 accepted）
    assert!(!is_valid_transition("opened", "processing"));
    // opened 不能直接跳到 resolved
    assert!(!is_valid_transition("opened", "resolved"));
}
