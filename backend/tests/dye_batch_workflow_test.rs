//! P0-T02 染整全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + 纯函数状态机校验（validate_lifecycle_status / validate_transition_code /
//! is_terminal_status / is_valid_transition / get_allowed_transitions / validate_transition_with_rule）
//! 染整状态机有 14 种状态 + 13 种流转码 + 30+ 条合法边，是测试重点。

#[cfg(test)]
mod tests {
    use bingxi_backend::models::status::dye_batch_lifecycle_status as status;
    use bingxi_backend::models::status::dye_batch_transition_code as code;
    use bingxi_backend::services::dye_batch_state_machine_service::{
        get_allowed_transitions, is_terminal_status, is_valid_transition,
        validate_lifecycle_status, validate_transition_code, validate_transition_with_rule,
    };

    // ===== 状态常量值正确性 =====

    /// 测试_染整状态常量_14种状态值正确性
    ///
    /// 验证染整生命周期 14 种状态常量值符合预期（小写风格）。
    #[test]
    fn 测试_染整状态常量_14种状态值正确性() {
        assert_eq!(status::PENDING_SCHEDULE, "pending_schedule");
        assert_eq!(status::SCHEDULED, "scheduled");
        assert_eq!(status::PREPARING, "preparing");
        assert_eq!(status::DYEING, "dyeing");
        assert_eq!(status::WASHING, "washing");
        assert_eq!(status::FIXING, "fixing");
        assert_eq!(status::DEHYDRATING, "dehydrating");
        assert_eq!(status::DRYING, "drying");
        assert_eq!(status::INSPECTING, "inspecting");
        assert_eq!(status::STORED, "stored");
        assert_eq!(status::SHIPPED, "shipped");
        assert_eq!(status::CANCELLED, "cancelled");
        assert_eq!(status::TERMINATED, "terminated");
        assert_eq!(status::REWORK, "rework");
    }

    /// 测试_染整流转码_13种代码值正确性
    ///
    /// 验证染整流转操作 13 种代码常量值符合预期。
    #[test]
    fn 测试_染整流转码_13种代码值正确性() {
        assert_eq!(code::SCHEDULE, "schedule");
        assert_eq!(code::PREPARE, "prepare");
        assert_eq!(code::START_DYEING, "start_dyeing");
        assert_eq!(code::WASH, "wash");
        assert_eq!(code::FIX, "fix");
        assert_eq!(code::DEHYDRATE, "dehydrate");
        assert_eq!(code::DRY, "dry");
        assert_eq!(code::INSPECT, "inspect");
        assert_eq!(code::STORE, "store");
        assert_eq!(code::SHIP, "ship");
        assert_eq!(code::CANCEL, "cancel");
        assert_eq!(code::REWORK, "rework");
        assert_eq!(code::TERMINATE, "terminate");
    }

    // ===== validate_lifecycle_status 状态校验 =====

    /// 测试_validate_lifecycle_status_合法状态通过
    #[test]
    fn 测试_validate_lifecycle_status_合法状态通过() {
        for s in [
            status::PENDING_SCHEDULE,
            status::SCHEDULED,
            status::PREPARING,
            status::DYEING,
            status::WASHING,
            status::FIXING,
            status::DEHYDRATING,
            status::DRYING,
            status::INSPECTING,
            status::STORED,
            status::SHIPPED,
            status::CANCELLED,
            status::TERMINATED,
            status::REWORK,
        ] {
            assert!(
                validate_lifecycle_status(s).is_ok(),
                "合法状态 {} 应通过校验",
                s
            );
        }
    }

    /// 测试_validate_lifecycle_status_非法状态失败
    #[test]
    fn 测试_validate_lifecycle_status_非法状态失败() {
        assert!(validate_lifecycle_status("unknown").is_err());
        assert!(validate_lifecycle_status("").is_err());
        assert!(validate_lifecycle_status("DYEING").is_err()); // 大写不匹配
    }

    // ===== validate_transition_code 流转码校验 =====

    /// 测试_validate_transition_code_合法码通过
    #[test]
    fn 测试_validate_transition_code_合法码通过() {
        for c in [
            code::SCHEDULE,
            code::PREPARE,
            code::START_DYEING,
            code::WASH,
            code::FIX,
            code::DEHYDRATE,
            code::DRY,
            code::INSPECT,
            code::STORE,
            code::SHIP,
            code::CANCEL,
            code::REWORK,
            code::TERMINATE,
        ] {
            assert!(
                validate_transition_code(c).is_ok(),
                "合法流转码 {} 应通过校验",
                c
            );
        }
    }

    /// 测试_validate_transition_code_非法码失败
    #[test]
    fn 测试_validate_transition_code_非法码失败() {
        assert!(validate_transition_code("unknown").is_err());
        assert!(validate_transition_code("").is_err());
    }

    // ===== is_terminal_status 终态判定 =====

    /// 测试_is_terminal_status_终态返回true
    #[test]
    fn 测试_is_terminal_status_终态返回true() {
        assert!(is_terminal_status(status::SHIPPED));
        assert!(is_terminal_status(status::CANCELLED));
        assert!(is_terminal_status(status::TERMINATED));
    }

    /// 测试_is_terminal_status_非终态返回false
    #[test]
    fn 测试_is_terminal_status_非终态返回false() {
        assert!(!is_terminal_status(status::PENDING_SCHEDULE));
        assert!(!is_terminal_status(status::SCHEDULED));
        assert!(!is_terminal_status(status::DYEING));
        assert!(!is_terminal_status(status::STORED));
        assert!(!is_terminal_status(status::REWORK));
    }

    // ===== is_valid_transition 合法流转判定 =====

    /// 测试_is_valid_transition_合法流转返回true
    ///
    /// 验证关键流转边：pending_schedule→scheduled、dyeing→washing、inspecting→stored 等。
    #[test]
    fn 测试_is_valid_transition_合法流转返回true() {
        assert!(is_valid_transition(
            Some(status::PENDING_SCHEDULE),
            status::SCHEDULED,
            code::SCHEDULE,
        ));
        assert!(is_valid_transition(
            Some(status::SCHEDULED),
            status::PREPARING,
            code::PREPARE,
        ));
        assert!(is_valid_transition(
            Some(status::PREPARING),
            status::DYEING,
            code::START_DYEING,
        ));
        assert!(is_valid_transition(
            Some(status::DYEING),
            status::WASHING,
            code::WASH,
        ));
        assert!(is_valid_transition(
            Some(status::INSPECTING),
            status::STORED,
            code::INSPECT,
        ));
        assert!(is_valid_transition(
            Some(status::STORED),
            status::SHIPPED,
            code::SHIP,
        ));
    }

    /// 测试_is_valid_transition_非法流转返回false
    ///
    /// 验证非法流转边：pending_schedule→shipped（跨状态）、dyeing→stored（跳过皂洗）等。
    #[test]
    fn 测试_is_valid_transition_非法流转返回false() {
        // 跨状态流转
        assert!(!is_valid_transition(
            Some(status::PENDING_SCHEDULE),
            status::SHIPPED,
            code::SHIP,
        ));
        // 跳过中间状态
        assert!(!is_valid_transition(
            Some(status::DYEING),
            status::STORED,
            code::STORE,
        ));
        // 终态不可再流转
        assert!(!is_valid_transition(
            Some(status::SHIPPED),
            status::STORED,
            code::INSPECT,
        ));
    }

    // ===== get_allowed_transitions 允许流转查询 =====

    /// 测试_get_allowed_transitions_待排缸状态允许2条边
    ///
    /// 验证 pending_schedule 状态允许流转到 scheduled 或 cancelled。
    #[test]
    fn 测试_get_allowed_transitions_待排缸状态允许2条边() {
        let transitions = get_allowed_transitions(status::PENDING_SCHEDULE);
        assert!(
            transitions.len() >= 2,
            "pending_schedule 至少应允许 2 条边（schedule + cancel），实际：{}",
            transitions.len()
        );
    }

    /// 测试_get_allowed_transitions_终态返回空列表
    ///
    /// 验证终态（shipped/cancelled/terminated）的允许流转列表为空。
    #[test]
    fn 测试_get_allowed_transitions_终态返回空列表() {
        assert!(
            get_allowed_transitions(status::SHIPPED).is_empty(),
            "终态 shipped 应无允许流转"
        );
        assert!(
            get_allowed_transitions(status::CANCELLED).is_empty(),
            "终态 cancelled 应无允许流转"
        );
        assert!(
            get_allowed_transitions(status::TERMINATED).is_empty(),
            "终态 terminated 应无允许流转"
        );
    }

    // ===== validate_transition_with_rule 带规则的流转校验 =====

    /// 测试_validate_transition_with_rule_合法流转通过
    #[test]
    fn 测试_validate_transition_with_rule_合法流转通过() {
        assert!(
            validate_transition_with_rule(
                Some(status::PENDING_SCHEDULE),
                status::SCHEDULED,
                code::SCHEDULE,
            )
            .is_ok()
        );
        assert!(
            validate_transition_with_rule(
                Some(status::DYEING),
                status::WASHING,
                code::WASH,
            )
            .is_ok()
        );
    }

    /// 测试_validate_transition_with_rule_非法流转失败
    #[test]
    fn 测试_validate_transition_with_rule_非法流转失败() {
        assert!(
            validate_transition_with_rule(
                Some(status::PENDING_SCHEDULE),
                status::SHIPPED,
                code::SHIP,
            )
            .is_err()
        );
        assert!(
            validate_transition_with_rule(
                Some(status::SHIPPED),
                status::STORED,
                code::INSPECT,
            )
            .is_err()
        );
    }

    // ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

    /// 集成测试：染整全流程 pending_schedule → scheduled → preparing → dyeing → washing →
    /// fixing → dehydrating → drying → inspecting → stored → shipped
    ///
    /// 需要 PostgreSQL + 前置缸号/流转卡数据。
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库 + 前置缸号/流转卡数据"]
    async fn 测试_染整全流程_待排缸到发货() {
        // 完整流程需 DyeBatchLifecycleLogService + DyeBatchOperationService 协同，
        // 留待真实环境验证。
    }
}
