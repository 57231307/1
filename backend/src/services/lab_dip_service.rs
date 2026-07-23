//! 化验室打样 Service（facade）
//!
//! v14 批次 423B：化验室打样流程贯通
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环 + §11.1.1 染色技术卡
//! 真实业务流程：打样通知单 → 打样（ABCD 多版样）→ 色样确认（OK 样）→ 复样 → 建数据库
//!
//! 核心能力：
//! - 打样通知单 CRUD + 状态流转（pending → sampling → submitted → approved/rejected → completed）
//! - 打样小样 CRUD + ABCD 多版样管理 + 对色结果记录
//! - OK 样确认（客户从多版中选 1 版，状态 → selected）
//! - 复样记录 CRUD + 复样结果判定（passed/failed/adjusted）
//! - 染色技术卡开具（复样通过后由研发组长开卡）
//!
//! 批次 D10 拆分：本文件作为 facade，保留 Service struct 定义 + new 构造函数 + 纯函数 + 测试。
//! 3 个 Service 的业务 impl 块迁移至 `lab_dip_ops` 子模块（request / sample / resample）。
//! DTO struct 迁移至 `lab_dip_ops::types`，本 facade 通过 `pub use` 二次 re-export 保持外部引用路径不变。
//! `db` 字段与跨模块纯函数声明为 `pub(crate)` 以便 ops 子模块访问。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::status::lab_dip_request as req_status;
use crate::utils::error::AppError;

// re-export DTOs 与 ops 子模块，保持外部 `use crate::services::lab_dip_service::{...}` 路径不变
pub use crate::services::lab_dip_ops::{
    CreateLabDipRequestRequest, CreateLabDipSampleRequest, CreateResampleRequest,
    IssueTechCardRequest, LabDipRequestQuery, RecordMatchingResultRequest,
    RecordResampleResultRequest, UpdateLabDipRequestRequest, UpdateLabDipSampleRequest,
};

/// 色差等级阈值（真实业务：4-5 级为 OK，<4 级为重打）
pub(crate) const COLOR_DIFF_OK_GRADE: i32 = 4;

// ============================================================================
// 打样通知单 Service struct 定义（业务 impl 块在 lab_dip_ops::request）
// ============================================================================

/// 打样通知单 Service
pub struct LabDipRequestService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl LabDipRequestService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成打样通知单号：LD-YYYYMMDDHHMMSS-NNN
    pub(crate) fn generate_request_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("LD-{}-{:03}", timestamp, random)
    }

    // ===== 状态流转校验（纯函数）=====

    /// 校验状态流转合法性
    ///
    /// 状态机：pending → sampling → submitted → approved/rejected → completed
    ///         rejected → sampling（重新打样）
    ///         approved → completed（复样通过后建库）
    pub fn validate_status_transition(current: &str, new: &str) -> Result<(), AppError> {
        let valid = match current {
            req_status::PENDING => matches!(new, req_status::SAMPLING),
            req_status::SAMPLING => matches!(new, req_status::SUBMITTED),
            req_status::SUBMITTED => matches!(new, req_status::APPROVED | req_status::REJECTED),
            req_status::REJECTED => matches!(new, req_status::SAMPLING),
            req_status::APPROVED => matches!(new, req_status::COMPLETED),
            req_status::COMPLETED => false, // 终态
            _ => false,
        };
        if !valid {
            return Err(AppError::business(format!(
                "打样通知单状态流转非法：{} → {}",
                current, new
            )));
        }
        Ok(())
    }

    /// 校验：仅 pending/sampling 状态可更新
    pub fn validate_can_update(status: &str) -> Result<(), AppError> {
        if !matches!(status, req_status::PENDING | req_status::SAMPLING) {
            return Err(AppError::business(format!(
                "当前状态 {} 不可更新（仅 pending/sampling 可更新）",
                status
            )));
        }
        Ok(())
    }

    /// 校验：仅 pending 状态可删除
    pub fn validate_can_delete(status: &str) -> Result<(), AppError> {
        if status != req_status::PENDING {
            return Err(AppError::business(format!(
                "当前状态 {} 不可删除（仅 pending 可删除）",
                status
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 打样小样 Service struct 定义（业务 impl 块在 lab_dip_ops::sample）
// ============================================================================

/// 打样小样 Service
pub struct LabDipSampleService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl LabDipSampleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 根据序号生成版本标识：1→A, 2→B, 3→C, 4→D, 5→E...
    pub(crate) fn label_from_seq(seq: i32) -> String {
        let c = ((seq - 1) as u8 + b'A') as char;
        c.to_string()
    }
}

// ============================================================================
// 复样记录 Service struct 定义（业务 impl 块在 lab_dip_ops::resample）
// ============================================================================

/// 复样记录 Service
pub struct LabDipResampleService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl LabDipResampleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成复样单号：RS-YYYYMMDDHHMMSS-NNN
    pub(crate) fn generate_resample_no() -> String {
        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_6_digit() % 1000;
        format!("RS-{}-{:03}", timestamp, random)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::SAMPLING).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::SAMPLING, req_status::SUBMITTED).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::SUBMITTED, req_status::APPROVED).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::SUBMITTED, req_status::REJECTED).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::REJECTED, req_status::SAMPLING).is_ok());
        assert!(LabDipRequestService::validate_status_transition(req_status::APPROVED, req_status::COMPLETED).is_ok());
    }

    /// 测试打样通知单状态流转非法
    #[test]
    fn test_request_status_transition_invalid() {
        // 非法流转
        assert!(LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::SUBMITTED).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::PENDING, req_status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::SAMPLING, req_status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::APPROVED, req_status::SAMPLING).is_err());
        assert!(LabDipRequestService::validate_status_transition(req_status::COMPLETED, req_status::SAMPLING).is_err());
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
}
