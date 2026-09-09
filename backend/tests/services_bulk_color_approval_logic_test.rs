//! 大货批色审批服务集成测试（纯逻辑层：色差判定 + 审批状态机）
//!
//! 覆盖缺口：bulk_color_approval_service.rs 在 CI 覆盖率报告中 0% 行覆盖，
//! 但其色差判定（evaluate_delta_e）与审批状态机（ApprovalStatus）是发货
//! 门禁的核心——状态机语义错误会导致不合格批色布直接发货。
//!
//! 验证范围：
//! - evaluate_delta_e 三级判定边界（≤1.2 通过 / ≤2.5 让步 / >2.5 拒绝）
//! - 高光敏感阈值切换（0.8）
//! - ApprovalStatus 终态判定与发货门禁（仅 approved 解除）
//! - FromStr 反序列化契约（DB 存储值往返）

use bingxi_backend::services::bulk_color_approval_service::{
    evaluate_delta_e, ApprovalStatus, DeltaEResult,
};
use std::str::FromStr;

#[test]
fn test_evaluate_delta_e_standard_thresholds() {
    // 标准阈值：≤1.2 通过 / 1.2-2.5 让步 / >2.5 拒绝（边界值全测）
    assert_eq!(evaluate_delta_e(0.0, false), DeltaEResult::Pass);
    assert_eq!(evaluate_delta_e(1.2, false), DeltaEResult::Pass);
    assert_eq!(
        evaluate_delta_e(1.21, false),
        DeltaEResult::ConditionalAccept
    );
    assert_eq!(
        evaluate_delta_e(2.5, false),
        DeltaEResult::ConditionalAccept
    );
    assert_eq!(evaluate_delta_e(2.51, false), DeltaEResult::Reject);
    assert_eq!(evaluate_delta_e(10.0, false), DeltaEResult::Reject);
}

#[test]
fn test_evaluate_delta_e_high_light_sensitive() {
    // 高光敏感阈值切换：通过线从 1.2 收紧到 0.8
    assert_eq!(evaluate_delta_e(0.8, true), DeltaEResult::Pass);
    assert_eq!(
        evaluate_delta_e(0.81, true),
        DeltaEResult::ConditionalAccept
    );
    // 让步线 2.5 不受高光敏感影响
    assert_eq!(evaluate_delta_e(2.5, true), DeltaEResult::ConditionalAccept);
    assert_eq!(evaluate_delta_e(2.51, true), DeltaEResult::Reject);
}

#[test]
fn test_approval_status_terminal_states() {
    // 终态：approved/rejected/downgraded/scrapped；非终态：pending/sampled/sent_to_customer/rework
    assert!(ApprovalStatus::Approved.is_terminal());
    assert!(ApprovalStatus::Rejected.is_terminal());
    assert!(ApprovalStatus::Downgraded.is_terminal());
    assert!(ApprovalStatus::Scrapped.is_terminal());
    assert!(!ApprovalStatus::Pending.is_terminal());
    assert!(!ApprovalStatus::Sampled.is_terminal());
    assert!(!ApprovalStatus::SentToCustomer.is_terminal());
    assert!(!ApprovalStatus::Rework.is_terminal());
}

#[test]
fn test_approval_status_delivery_gate() {
    // 发货门禁核心语义：仅 approved 解除；downgraded/scrapped 虽终态仍阻断
    assert!(ApprovalStatus::Approved.unblocks_delivery());
    assert!(!ApprovalStatus::Downgraded.unblocks_delivery());
    assert!(!ApprovalStatus::Scrapped.unblocks_delivery());
    assert!(!ApprovalStatus::Rejected.unblocks_delivery());
    assert!(!ApprovalStatus::Rework.unblocks_delivery());
    assert!(!ApprovalStatus::Pending.unblocks_delivery());
}

#[test]
fn test_approval_status_from_str_roundtrip() {
    // DB 存储值往返：as_str → from_str 必须还原同一状态（8 态全覆盖）
    let all = [
        ApprovalStatus::Pending,
        ApprovalStatus::Sampled,
        ApprovalStatus::SentToCustomer,
        ApprovalStatus::Approved,
        ApprovalStatus::Rejected,
        ApprovalStatus::Rework,
        ApprovalStatus::Downgraded,
        ApprovalStatus::Scrapped,
    ];
    for st in all {
        assert_eq!(ApprovalStatus::from_str(st.as_str()).unwrap(), st);
    }
    // 非法值必须报错（fail-secure，不可静默解析为默认态）
    assert!(ApprovalStatus::from_str("unknown").is_err());
    assert!(ApprovalStatus::from_str("").is_err());
    assert!(ApprovalStatus::from_str("APPROVED").is_err()); // 大小写敏感：防 DB 脏数据静默通过
}
