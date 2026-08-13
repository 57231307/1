use bingxi_backend::models::request::req_status;
use bingxi_backend::models::status::quality_dyeing::*;
use bingxi_backend::services::lab_dip_request_service::LabDipRequestService;
use bingxi_backend::services::quality_dyeing;

/// 测试版本标识生成：1→A, 2→B, 3→C, 4→D
#[test]
fn test_label_from_seq() {
    assert_eq!(LabDipSampleService::label_from_seq(1), "A");
    assert_eq!(LabDipSampleService::label_from_seq(2), "B");
    assert_eq!(LabDipSampleService::label_from_seq(3), "C");
    assert_eq!(LabDipSampleService::label_from_seq(4), "D");
    assert_eq!(LabDipSampleService::label_from_seq(5), "E");
}

/// 测试打样通知单状态流转合法性
#[test]
fn test_request_status_transition_valid() {
    // 合法流转
    assert!(
        LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::SAMPLING)
            .is_ok()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::SAMPLING,
            req_status::SUBMITTED
        )
        .is_ok()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::SUBMITTED,
            req_status::APPROVED
        )
        .is_ok()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::SUBMITTED,
            req_status::REJECTED
        )
        .is_ok()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::REJECTED,
            req_status::SAMPLING
        )
        .is_ok()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::APPROVED,
            req_status::COMPLETED
        )
        .is_ok()
    );
}

/// 测试打样通知单状态流转非法
#[test]
fn test_request_status_transition_invalid() {
    // 非法流转
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::PENDING,
            req_status::SUBMITTED
        )
        .is_err()
    );
    assert!(
        LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::APPROVED)
            .is_err()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::SAMPLING,
            req_status::APPROVED
        )
        .is_err()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::APPROVED,
            req_status::SAMPLING
        )
        .is_err()
    );
    assert!(
        LabDipRequestService::validate_status_transition(
            req_status::COMPLETED,
            req_status::SAMPLING
        )
        .is_err()
    );
}

/// 测试通知单更新状态校验
#[test]
fn test_validate_can_update() {
    assert!(LabDipRequestService::validate_can_update(req_status::PENDING).is_ok());
    assert!(LabDipRequestService::validate_can_update(req_status::SAMPLING).is_ok());
    assert!(LabDipRequestService::validate_can_update(req_status::SUBMITTED).is_err());
    assert!(LabDipRequestService::validate_can_update(req_status::APPROVED).is_err());
    assert!(LabDipRequestService::validate_can_update(req_status::COMPLETED).is_err());
}

/// 测试通知单删除状态校验
#[test]
fn test_validate_can_delete() {
    assert!(LabDipRequestService::validate_can_delete(req_status::PENDING).is_ok());
    assert!(LabDipRequestService::validate_can_delete(req_status::SAMPLING).is_err());
    assert!(LabDipRequestService::validate_can_delete(req_status::APPROVED).is_err());
}

/// 测试打样通知单号生成格式
#[test]
fn test_generate_request_no() {
    let no = LabDipRequestService::generate_request_no();
    assert!(no.starts_with("LD-"));
    let parts: Vec<&str> = no.split('-').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
    assert_eq!(parts[2].len(), 3); // 3 位随机
}

/// 测试复样单号生成格式
#[test]
fn test_generate_resample_no() {
    let no = LabDipResampleService::generate_resample_no();
    assert!(no.starts_with("RS-"));
    let parts: Vec<&str> = no.split('-').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[1].len(), 14);
    assert_eq!(parts[2].len(), 3);
}
