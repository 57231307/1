//! 导出审批令牌生命周期（m_export_approval_lifecycle）纯逻辑与词表单源锁定测试
//!
//! 覆盖生命周期四条的判定内核（无库、注入时钟，确定性）：
//!   1. 一个令牌只放行被批准的那一份数据范围（词表与资源类型单源，见本文件 vocab 段）；
//!   2. 导出成功即作废（Consumed 终态判定）；
//!   3. 未导出成功后 5 分钟窗口内禁止重试（WindowNotReached）；
//!   4. 未成功前提下最多重试 3 次（RetriesExhausted）。
//! retry_state_of 的入参时钟显式注入，不真等 5 分钟，边界确定收敛。
//!
//! 通道：纯逻辑用例，无需 test_common 真库夹具（不触库）。DB 侧的受理与消费回写链
//! 由 services_export_approval_service_test.rs 与 export_approval_enforce_test.rs 承担，
//! 完整终审签发/消费/重放的活体链在活体测试文件中标「由 CI 兑付」。

use bingxi_backend::models::export_approval_request::{
    ApprovalStatus, Model, RiskLevel, sensitive_resources,
};
use bingxi_backend::services::export_approval_service::{
    ExportApprovalService, RETRY_WINDOW_MINUTES, TokenRetryState,
};
use chrono::{DateTime, FixedOffset, TimeZone, Utc};

/// 构造一个可判定 retry_state_of 的申请行骨架（其余字段取中性值，只覆盖被测字段）
fn base_model(status: &str) -> Model {
    let zero: DateTime<FixedOffset> = Utc.timestamp_opt(0, 0).unwrap().fixed_offset();
    Model {
        id: 1,
        applicant_user_id: 1,
        applicant_username: "tester".to_string(),
        approver_user_id: None,
        approver_username: None,
        resource_type: "customer".to_string(),
        export_params: None,
        estimated_rows: Some(0),
        file_format: "xlsx".to_string(),
        status: status.to_string(),
        approval_level: 1,
        approver_comments: None,
        approved_at: None,
        rejected_at: None,
        download_token: None,
        token_expires_at: None,
        download_count: 0,
        max_downloads: 1,
        file_path: None,
        file_size_bytes: None,
        file_checksum: None,
        applicant_ip: None,
        approver_ip: None,
        applicant_user_agent: None,
        risk_level: RiskLevel::Low.as_str().to_string(),
        context: None,
        created_at: zero,
        updated_at: zero,
        completed_at: None,
        token_hash: None,
        token_fingerprint: None,
        token_viewed: false,
        retry_count: 0,
        last_attempt_at: None,
        max_retries: 3,
        token_consumed: false,
        token_consumed_at: None,
        bpm_instance_id: None,
    }
}

fn utc(offset_seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(offset_seconds, 0).unwrap()
}

// ===== 生命周期条件 3：未到 5 分钟窗口禁止重试 =====

#[test]
fn window_not_reached_blocks_retry_before_five_minutes() {
    // 上次尝试在窗口起点（epoch+0），当前时刻距其 4 分钟 → 窗口未到
    let mut m = base_model(ApprovalStatus::Approved.as_str());
    m.last_attempt_at = Some(utc(0).fixed_offset());
    let now = utc(RETRY_WINDOW_MINUTES * 60 - 60); // 差 1 分钟到窗口
    match ExportApprovalService::retry_state_of(&m, now) {
        TokenRetryState::WindowNotReached { remaining_secs } => {
            assert_eq!(
                remaining_secs, 60,
                "窗口差 1 分钟应剩 60 秒，实际 {remaining_secs}"
            );
        }
        other => panic!("窗口未到应判 WindowNotReached，实际 {other:?}"),
    }
}

#[test]
fn window_elapsed_allows_retry_at_boundary() {
    // 5 分钟整点按「窗口已过、允许重试」确定收敛
    let mut m = base_model(ApprovalStatus::Approved.as_str());
    m.last_attempt_at = Some(utc(0).fixed_offset());
    let now = utc(RETRY_WINDOW_MINUTES * 60); // 恰好满 5 分钟
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, now),
        TokenRetryState::Eligible,
        "满 5 分钟边界应放行重试"
    );
}

// ===== 生命周期条件 4：未成功前提下最多重试 3 次，超限被拒 =====

#[test]
fn retries_exhausted_when_count_reaches_max() {
    // retry_count 已达 max_retries(3)，即便窗口已过也应判用尽，不再给等待提示
    let mut m = base_model(ApprovalStatus::Approved.as_str());
    m.last_attempt_at = Some(utc(0).fixed_offset());
    m.retry_count = 3;
    m.max_retries = 3;
    let now = utc(RETRY_WINDOW_MINUTES * 60 * 10); // 窗口远过
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, now),
        TokenRetryState::RetriesExhausted,
        "重试用尽应判 RetriesExhausted"
    );
}

#[test]
fn within_retry_budget_still_eligible_after_window() {
    let mut m = base_model(ApprovalStatus::Approved.as_str());
    m.last_attempt_at = Some(utc(0).fixed_offset());
    m.retry_count = 2; // 尚未到 3
    m.max_retries = 3;
    let now = utc(RETRY_WINDOW_MINUTES * 60); // 窗口刚到
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, now),
        TokenRetryState::Eligible,
        "额度未尽且窗口已过应放行"
    );
}

// ===== 生命周期条件 2：导出成功后令牌作废，重放被拒 =====

#[test]
fn consumed_token_flag_yields_consumed_state() {
    let mut m = base_model(ApprovalStatus::Approved.as_str());
    m.token_consumed = true;
    let now = utc(0);
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, now),
        TokenRetryState::Consumed,
        "token_consumed=true 必须判 Consumed（成功导出后重放的前置判定）"
    );
}

#[test]
fn consumed_status_yields_consumed_state() {
    let m = base_model(ApprovalStatus::Consumed.as_str());
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, utc(0)),
        TokenRetryState::Consumed,
        "status=consumed 终态必须判 Consumed"
    );
}

// ===== 有效期边界：过硬有效期判 Expired（防重试永生） =====

#[test]
fn expired_by_clock_when_past_expiry() {
    let mut m = base_model(ApprovalStatus::Approved.as_str());
    m.token_expires_at = Some(utc(0).fixed_offset());
    let now = utc(1); // 过期 1 秒
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, now),
        TokenRetryState::Expired,
        "超过 token_expires_at 应判 Expired"
    );
}

#[test]
fn fresh_approved_token_is_eligible() {
    let m = base_model(ApprovalStatus::Approved.as_str());
    assert_eq!(
        ExportApprovalService::retry_state_of(&m, utc(0)),
        TokenRetryState::Eligible,
        "首次签发、无尝试史的已批准令牌应放行"
    );
}

// ===== 状态词表单源：八值 as_str / parse_status 双向闭合，与 DDL CHECK 逐项一致 =====

#[test]
fn approval_status_vocab_round_trip_is_closed() {
    let vocab = [
        ("pending", ApprovalStatus::Pending),
        ("pending_l2", ApprovalStatus::PendingL2),
        ("approved", ApprovalStatus::Approved),
        ("rejected", ApprovalStatus::Rejected),
        ("expired", ApprovalStatus::Expired),
        ("cancelled", ApprovalStatus::Cancelled),
        ("consumed", ApprovalStatus::Consumed),
        ("exhausted", ApprovalStatus::Exhausted),
    ];
    for (token, variant) in vocab {
        assert_eq!(variant.as_str(), token, "as_str 取值漂移：{variant:?}");
        assert_eq!(
            ApprovalStatus::parse_status(token),
            Some(variant),
            "parse_status 反解漂移：{token}"
        );
    }
    // 词表外取值必须解析失败（不得被静默兜底成某个默认态）
    assert_eq!(ApprovalStatus::parse_status("consumed "), None);
    assert_eq!(ApprovalStatus::parse_status("in_progress"), None);
    assert_eq!(ApprovalStatus::parse_status(""), None);
}

#[test]
fn terminal_and_pending_predicates_partition_vocab() {
    let terminal = [
        ApprovalStatus::Rejected,
        ApprovalStatus::Expired,
        ApprovalStatus::Cancelled,
        ApprovalStatus::Consumed,
        ApprovalStatus::Exhausted,
    ];
    for s in terminal {
        assert!(s.is_terminal(), "{s:?} 应为终态");
        assert!(!s.is_pending_any(), "{s:?} 不应是在途态");
    }
    for s in [ApprovalStatus::Pending, ApprovalStatus::PendingL2] {
        assert!(s.is_pending_any(), "{s:?} 应是在途态");
        assert!(!s.is_terminal(), "{s:?} 不应是终态");
    }
    assert!(!ApprovalStatus::Approved.is_terminal());
    assert!(!ApprovalStatus::Approved.is_pending_any());
}

// ===== 资源类型词表单源：sensitive_resources 注册表与判定一致 =====

#[test]
fn sensitive_resources_registry_is_single_source() {
    for rt in sensitive_resources::all_sensitive() {
        assert!(
            sensitive_resources::is_sensitive(rt),
            "注册表内的 {rt} 必须判敏感"
        );
    }
    // 清单外不得判敏感（防止词表外资源被误纳入审批链）
    assert!(!sensitive_resources::is_sensitive("inventory"));
    assert!(!sensitive_resources::is_sensitive("warehouse"));
}

// ===== 令牌哈希：签发即只可复算哈希，指纹不构成可用凭据 =====

#[test]
fn sha256_hex_matches_known_vector_and_is_lowercase() {
    // "abc" 的 SHA-256 标准向量，钉住哈希实现（校验侧与签发侧同源复算的前提）
    assert_eq!(
        ExportApprovalService::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn issued_token_hash_and_prefix_fingerprint_are_consistent() {
    let issued = ExportApprovalService::issue_token();
    assert!(issued.plaintext.starts_with("exp_"), "明文前缀应为 exp_");
    assert_eq!(
        issued.hash,
        ExportApprovalService::sha256_hex(issued.plaintext.as_bytes()),
        "落库哈希必须是对明文的同函数复算"
    );
    assert_eq!(
        issued.fingerprint,
        issued.plaintext.chars().take(12).collect::<String>(),
        "指纹应为明文前 12 字符"
    );
    // 指纹不得等于完整明文（掩码后不构成可转发凭据）
    assert_ne!(issued.fingerprint, issued.plaintext);
}
