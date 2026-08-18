//! 售后状态机纯函数测试（V15 P1 batch-19）
//!
//! 复现 custom_order_aftersales_service::is_valid_transition 的状态转换规则。
//! 实现为私有函数，此处本地复现规则表进行验证。

fn is_valid_transition(from: &str, to: &str) -> bool {
    use std::collections::HashMap;
    let mut valid: HashMap<&str, Vec<&str>> = HashMap::new();
    valid.insert("opened", vec!["accepted", "rejected", "closed"]);
    valid.insert("accepted", vec!["processing", "rejected", "closed"]);
    valid.insert("processing", vec!["resolved", "closed", "rejected"]);
    valid.insert("resolved", vec!["evaluated", "closed"]);
    valid.insert("evaluated", vec!["closed"]);
    valid.insert("closed", vec![]);
    valid.insert("rejected", vec![]);

    valid.get(from).map(|v| v.contains(&to)).unwrap_or(false)
}

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
