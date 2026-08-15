use bingxi_backend::services::color_card_issue_service::*;
use std::str::FromStr;

#[test]
fn test_issue_status_as_str_qbztys() {
    assert_eq!(IssueStatus::Issued.as_str(), "issued");
    assert_eq!(IssueStatus::Returned.as_str(), "returned");
    assert_eq!(IssueStatus::Lost.as_str(), "lost");
    assert_eq!(IssueStatus::Damaged.as_str(), "damaged");
    assert_eq!(IssueStatus::Cancelled.as_str(), "cancelled");
}

#[test]
fn test_issue_status_ztpd_ztfhtrue() {
    assert!(IssueStatus::Returned.is_terminal());
    assert!(IssueStatus::Lost.is_terminal());
    assert!(IssueStatus::Damaged.is_terminal());
    assert!(IssueStatus::Cancelled.is_terminal());
}

#[test]
fn test_issue_status_ztpd_fztfhfalse() {
    assert!(!IssueStatus::Issued.is_terminal());
}

#[test]
fn test_issue_status_from_str_hfzfcjxcg() {
    assert_eq!(
        IssueStatus::from_str("issued").unwrap(),
        IssueStatus::Issued
    );
    assert_eq!(
        IssueStatus::from_str("returned").unwrap(),
        IssueStatus::Returned
    );
    assert_eq!(IssueStatus::from_str("lost").unwrap(), IssueStatus::Lost);
    assert_eq!(
        IssueStatus::from_str("damaged").unwrap(),
        IssueStatus::Damaged
    );
    assert_eq!(
        IssueStatus::from_str("cancelled").unwrap(),
        IssueStatus::Cancelled
    );
}

#[test]
fn test_issue_status_from_str_ffzfcjxsb() {
    assert!(IssueStatus::from_str("unknown").is_err());
    assert!(IssueStatus::from_str("").is_err());
    assert!(IssueStatus::from_str("ISSUED").is_err());
    assert!(IssueStatus::from_str(" issued ").is_err());
}

#[test]
fn test_issue_status_xlhfxlhwfyz() {
    let all_statuses = [
        IssueStatus::Issued,
        IssueStatus::Returned,
        IssueStatus::Lost,
        IssueStatus::Damaged,
        IssueStatus::Cancelled,
    ];
    for status in all_statuses {
        let serialized = status.as_str();
        let parsed = IssueStatus::from_str(serialized).unwrap();
        assert_eq!(status, parsed, "状态 {:?} 序列化反序列化往返不一致", status);
    }
}

#[test]
fn test_issue_status_ztjwzx_ztslzq() {
    let all_statuses = [
        IssueStatus::Issued,
        IssueStatus::Returned,
        IssueStatus::Lost,
        IssueStatus::Damaged,
        IssueStatus::Cancelled,
    ];
    let terminal_count = all_statuses.iter().filter(|s| s.is_terminal()).count();
    assert_eq!(
        terminal_count, 4,
        "应有 4 个终态（returned/lost/damaged/cancelled）"
    );
    let non_terminal_count = all_statuses.iter().filter(|s| !s.is_terminal()).count();
    assert_eq!(non_terminal_count, 1, "应有 1 个非终态（issued）");
}
