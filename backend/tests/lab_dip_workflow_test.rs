//! P0-T02 打样全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + 纯函数状态机校验（validate_status_transition /
//! validate_can_update / validate_can_delete）
//! 打样状态机：PENDING → SAMPLING → SUBMITTED → APPROVED/REJECTED → COMPLETED

#[cfg(test)]
mod tests {
    use bingxi_backend::models::status::lab_dip_request as status;
    use bingxi_backend::services::lab_dip_service::LabDipRequestService;

    // ===== 状态常量值正确性 =====

    /// 测试_打样状态常量_值正确性
    ///
    /// 验证打样通知单 6 种状态常量值符合预期（大写风格）。
    #[test]
    fn 测试_打样状态常量_值正确性() {
        assert_eq!(status::PENDING, "PENDING");
        assert_eq!(status::SAMPLING, "SAMPLING");
        assert_eq!(status::SUBMITTED, "SUBMITTED");
        assert_eq!(status::APPROVED, "APPROVED");
        assert_eq!(status::REJECTED, "REJECTED");
        assert_eq!(status::COMPLETED, "COMPLETED");
    }

    /// 测试_打样状态常量_大写风格一致性
    #[test]
    fn 测试_打样状态常量_大写风格一致性() {
        for s in [
            status::PENDING,
            status::SAMPLING,
            status::SUBMITTED,
            status::APPROVED,
            status::REJECTED,
            status::COMPLETED,
        ] {
            assert!(
                s.chars().all(|c| c.is_uppercase() || c == '_'),
                "状态 {} 应全大写",
                s
            );
        }
    }

    // ===== validate_status_transition 状态流转校验 =====

    /// 测试_validate_status_transition_合法流转通过
    ///
    /// 验证合法流转边：PENDING→SAMPLING、SAMPLING→SUBMITTED、SUBMITTED→APPROVED 等。
    #[test]
    fn 测试_validate_status_transition_合法流转通过() {
        assert!(
            LabDipRequestService::validate_status_transition(status::PENDING, status::SAMPLING)
                .is_ok()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::SAMPLING, status::SUBMITTED)
                .is_ok()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::SUBMITTED, status::APPROVED)
                .is_ok()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::SUBMITTED, status::REJECTED)
                .is_ok()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::REJECTED, status::SAMPLING)
                .is_ok()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::APPROVED, status::COMPLETED)
                .is_ok()
        );
    }

    /// 测试_validate_status_transition_非法流转失败
    ///
    /// 验证非法流转边：PENDING→APPROVED（跳过打样）、COMPLETED→PENDING（终态回退）等。
    #[test]
    fn 测试_validate_status_transition_非法流转失败() {
        assert!(
            LabDipRequestService::validate_status_transition(status::PENDING, status::APPROVED)
                .is_err()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::COMPLETED, status::PENDING)
                .is_err()
        );
        assert!(
            LabDipRequestService::validate_status_transition(status::APPROVED, status::SAMPLING)
                .is_err()
        );
    }

    // ===== validate_can_update 可更新校验 =====

    /// 测试_validate_can_update_可更新状态通过
    ///
    /// 验证 PENDING 和 SAMPLING 状态允许更新。
    #[test]
    fn 测试_validate_can_update_可更新状态通过() {
        assert!(LabDipRequestService::validate_can_update(status::PENDING).is_ok());
        assert!(LabDipRequestService::validate_can_update(status::SAMPLING).is_ok());
    }

    /// 测试_validate_can_update_不可更新状态失败
    ///
    /// 验证 SUBMITTED/APPROVED/REJECTED/COMPLETED 状态不允许更新。
    #[test]
    fn 测试_validate_can_update_不可更新状态失败() {
        assert!(LabDipRequestService::validate_can_update(status::SUBMITTED).is_err());
        assert!(LabDipRequestService::validate_can_update(status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_can_update(status::REJECTED).is_err());
        assert!(LabDipRequestService::validate_can_update(status::COMPLETED).is_err());
    }

    // ===== validate_can_delete 可删除校验 =====

    /// 测试_validate_can_delete_仅PENDING可删除
    ///
    /// 验证只有 PENDING 状态允许删除。
    #[test]
    fn 测试_validate_can_delete_仅PENDING可删除() {
        assert!(LabDipRequestService::validate_can_delete(status::PENDING).is_ok());
    }

    /// 测试_validate_can_delete_非PENDING不可删除
    ///
    /// 验证 SAMPLING 及之后的状态不允许删除。
    #[test]
    fn 测试_validate_can_delete_非PENDING不可删除() {
        assert!(LabDipRequestService::validate_can_delete(status::SAMPLING).is_err());
        assert!(LabDipRequestService::validate_can_delete(status::SUBMITTED).is_err());
        assert!(LabDipRequestService::validate_can_delete(status::APPROVED).is_err());
        assert!(LabDipRequestService::validate_can_delete(status::COMPLETED).is_err());
    }

    // ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

    /// 集成测试：打样全流程 PENDING → SAMPLING → SUBMITTED → APPROVED → COMPLETED
    ///
    /// 需要 PostgreSQL + 前置客户/产品/颜色数据。
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库 + 前置客户/产品/颜色数据"]
    async fn 测试_打样全流程_待打样到完成() {
        // 完整流程需 LabDipRequestService + LabDipSampleService 协同，
        // 留待真实环境验证。
    }
}
