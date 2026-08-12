    use super::*;
#[cfg(test)]
mod tests {

    // ===== 校验纯函数测试 =====

    #[test]
    fn 测试校验生命周期状态_合法() {
        assert!(validate_lifecycle_status("pending_schedule").is_ok());
        assert!(validate_lifecycle_status("scheduled").is_ok());
        assert!(validate_lifecycle_status("preparing").is_ok());
        assert!(validate_lifecycle_status("dyeing").is_ok());
        assert!(validate_lifecycle_status("washing").is_ok());
        assert!(validate_lifecycle_status("fixing").is_ok());
        assert!(validate_lifecycle_status("dehydrating").is_ok());
        assert!(validate_lifecycle_status("drying").is_ok());
        assert!(validate_lifecycle_status("inspecting").is_ok());
        assert!(validate_lifecycle_status("stored").is_ok());
        assert!(validate_lifecycle_status("shipped").is_ok());
        assert!(validate_lifecycle_status("cancelled").is_ok());
        assert!(validate_lifecycle_status("terminated").is_ok());
        assert!(validate_lifecycle_status("rework").is_ok());
    }

    #[test]
    fn 测试校验生命周期状态_非法() {
        assert!(validate_lifecycle_status("invalid").is_err());
        assert!(validate_lifecycle_status("").is_err());
        assert!(validate_lifecycle_status("PENDING_SCHEDULE").is_err());
    }

    #[test]
    fn 测试校验流转操作代码_合法() {
        assert!(validate_transition_code("schedule").is_ok());
        assert!(validate_transition_code("prepare").is_ok());
        assert!(validate_transition_code("start_dyeing").is_ok());
        assert!(validate_transition_code("wash").is_ok());
        assert!(validate_transition_code("fix").is_ok());
        assert!(validate_transition_code("dehydrate").is_ok());
        assert!(validate_transition_code("dry").is_ok());
        assert!(validate_transition_code("inspect").is_ok());
        assert!(validate_transition_code("store").is_ok());
        assert!(validate_transition_code("ship").is_ok());
        assert!(validate_transition_code("cancel").is_ok());
        assert!(validate_transition_code("rework").is_ok());
        assert!(validate_transition_code("terminate").is_ok());
    }

    #[test]
    fn 测试校验流转操作代码_非法() {
        assert!(validate_transition_code("invalid").is_err());
        assert!(validate_transition_code("").is_err());
    }

    #[test]
    fn 测试校验回修类型_合法() {
        assert!(validate_rework_type("color_difference").is_ok());
        assert!(validate_rework_type("defect").is_ok());
        assert!(validate_rework_type("specification_unqualified").is_ok());
        assert!(validate_rework_type("re_dye").is_ok());
        assert!(validate_rework_type("replenish_dye").is_ok());
        assert!(validate_rework_type("other").is_ok());
    }

    #[test]
    fn 测试校验回修类型_非法() {
        assert!(validate_rework_type("invalid").is_err());
        assert!(validate_rework_type("").is_err());
    }

    #[test]
    fn 测试校验回修单状态_合法() {
        assert!(validate_rework_status("draft").is_ok());
        assert!(validate_rework_status("approved").is_ok());
        assert!(validate_rework_status("in_progress").is_ok());
        assert!(validate_rework_status("completed").is_ok());
        assert!(validate_rework_status("cancelled").is_ok());
    }

    #[test]
    fn 测试校验回修单状态_非法() {
        assert!(validate_rework_status("invalid").is_err());
        assert!(validate_rework_status("").is_err());
    }

    #[test]
    fn 测试校验操作类型_合法() {
        assert!(validate_operation_type("merge").is_ok());
        assert!(validate_operation_type("split").is_ok());
        assert!(validate_operation_type("priority_adjust").is_ok());
        assert!(validate_operation_type("batch_change").is_ok());
        assert!(validate_operation_type("schedule_change").is_ok());
        assert!(validate_operation_type("terminate").is_ok());
    }

    #[test]
    fn 测试校验操作类型_非法() {
        assert!(validate_operation_type("invalid").is_err());
        assert!(validate_operation_type("").is_err());
    }

    // ===== 终态判断测试 =====

    #[test]
    fn 测试终态判断_终态返回true() {
        assert!(is_terminal_status("shipped"));
        assert!(is_terminal_status("cancelled"));
        assert!(is_terminal_status("terminated"));
    }

    #[test]
    fn 测试终态判断_非终态返回false() {
        assert!(!is_terminal_status("pending_schedule"));
        assert!(!is_terminal_status("scheduled"));
        assert!(!is_terminal_status("preparing"));
        assert!(!is_terminal_status("dyeing"));
        assert!(!is_terminal_status("washing"));
        assert!(!is_terminal_status("fixing"));
        assert!(!is_terminal_status("dehydrating"));
        assert!(!is_terminal_status("drying"));
        assert!(!is_terminal_status("inspecting"));
        assert!(!is_terminal_status("stored"));
        assert!(!is_terminal_status("rework"));
    }

    // ===== 状态流转校验测试 =====

    #[test]
    fn 测试状态流转_合法流转() {
        // pending_schedule → scheduled（排缸）
        assert!(is_valid_transition(
            Some("pending_schedule"),
            "scheduled",
            "schedule"
        ));
        // scheduled → preparing（备布）
        assert!(is_valid_transition(
            Some("scheduled"),
            "preparing",
            "prepare"
        ));
        // preparing → dyeing（进缸染色）
        assert!(is_valid_transition(
            Some("preparing"),
            "dyeing",
            "start_dyeing"
        ));
        // dyeing → washing（皂洗）
        assert!(is_valid_transition(Some("dyeing"), "washing", "wash"));
        // washing → fixing（固色）
        assert!(is_valid_transition(Some("washing"), "fixing", "fix"));
        // fixing → dehydrating（脱水）
        assert!(is_valid_transition(
            Some("fixing"),
            "dehydrating",
            "dehydrate"
        ));
        // dehydrating → drying（烘干）
        assert!(is_valid_transition(Some("dehydrating"), "drying", "dry"));
        // drying → inspecting（验布）
        assert!(is_valid_transition(Some("drying"), "inspecting", "inspect"));
        // inspecting → stored（入库）
        assert!(is_valid_transition(Some("inspecting"), "stored", "store"));
        // stored → shipped（发货）
        assert!(is_valid_transition(Some("stored"), "shipped", "ship"));
        // inspecting → rework（回修）
        assert!(is_valid_transition(Some("inspecting"), "rework", "rework"));
        // stored → rework（回修）
        assert!(is_valid_transition(Some("stored"), "rework", "rework"));
        // rework → dyeing（回修重新进缸）
        assert!(is_valid_transition(
            Some("rework"),
            "dyeing",
            "start_dyeing"
        ));
    }

    #[test]
    fn 测试状态流转_取消流转合法() {
        // 任意非终态 → cancelled
        assert!(is_valid_transition(
            Some("pending_schedule"),
            "cancelled",
            "cancel"
        ));
        assert!(is_valid_transition(
            Some("scheduled"),
            "cancelled",
            "cancel"
        ));
        assert!(is_valid_transition(
            Some("preparing"),
            "cancelled",
            "cancel"
        ));
        assert!(is_valid_transition(Some("dyeing"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("washing"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("fixing"), "cancelled", "cancel"));
        assert!(is_valid_transition(
            Some("dehydrating"),
            "cancelled",
            "cancel"
        ));
        assert!(is_valid_transition(Some("drying"), "cancelled", "cancel"));
        assert!(is_valid_transition(
            Some("inspecting"),
            "cancelled",
            "cancel"
        ));
        assert!(is_valid_transition(Some("stored"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("rework"), "cancelled", "cancel"));
    }

    #[test]
    fn 测试状态流转_终止流转合法() {
        // scheduled/preparing/dyeing/rework → terminated
        assert!(is_valid_transition(
            Some("scheduled"),
            "terminated",
            "terminate"
        ));
        assert!(is_valid_transition(
            Some("preparing"),
            "terminated",
            "terminate"
        ));
        assert!(is_valid_transition(
            Some("dyeing"),
            "terminated",
            "terminate"
        ));
        assert!(is_valid_transition(
            Some("rework"),
            "terminated",
            "terminate"
        ));
    }

    #[test]
    fn 测试状态流转_终态不可流转() {
        // shipped 不可流转
        assert!(!is_valid_transition(Some("shipped"), "stored", "store"));
        assert!(!is_valid_transition(Some("shipped"), "cancelled", "cancel"));
        // cancelled 不可流转
        assert!(!is_valid_transition(
            Some("cancelled"),
            "scheduled",
            "schedule"
        ));
        assert!(!is_valid_transition(
            Some("cancelled"),
            "terminated",
            "terminate"
        ));
        // terminated 不可流转
        assert!(!is_valid_transition(
            Some("terminated"),
            "scheduled",
            "schedule"
        ));
        assert!(!is_valid_transition(
            Some("terminated"),
            "cancelled",
            "cancel"
        ));
    }

    #[test]
    fn 测试状态流转_非法流转() {
        // pending_schedule 不能直接到 dyeing
        assert!(!is_valid_transition(
            Some("pending_schedule"),
            "dyeing",
            "start_dyeing"
        ));
        // scheduled 不能直接到 washing
        assert!(!is_valid_transition(Some("scheduled"), "washing", "wash"));
        // dyeing 不能直接到 inspecting（必须经过 washing/fixing/dehydrating/drying）
        assert!(!is_valid_transition(
            Some("dyeing"),
            "inspecting",
            "inspect"
        ));
        // inspecting 不能直接到 shipped（必须经过 stored）
        assert!(!is_valid_transition(Some("inspecting"), "shipped", "ship"));
        // 操作代码不匹配
        assert!(!is_valid_transition(
            Some("pending_schedule"),
            "scheduled",
            "prepare"
        ));
    }

    #[test]
    fn test_state_transition_from_status_none_returns_false() {
        // from_status 为 None 时无匹配规则
        assert!(!is_valid_transition(None, "scheduled", "schedule"));
        assert!(!is_valid_transition(None, "cancelled", "cancel"));
    }

    // ===== 允许的流转列表测试 =====

    #[test]
    fn 测试获取允许流转_待排缸() {
        let transitions = get_allowed_transitions("pending_schedule");
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&("scheduled", "schedule")));
        assert!(transitions.contains(&("cancelled", "cancel")));
    }

    #[test]
    fn 测试获取允许流转_已排缸() {
        let transitions = get_allowed_transitions("scheduled");
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&("preparing", "prepare")));
        assert!(transitions.contains(&("cancelled", "cancel")));
        assert!(transitions.contains(&("terminated", "terminate")));
    }

    #[test]
    fn 测试获取允许流转_进缸染色() {
        let transitions = get_allowed_transitions("dyeing");
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&("washing", "wash")));
        assert!(transitions.contains(&("cancelled", "cancel")));
        assert!(transitions.contains(&("terminated", "terminate")));
    }

    #[test]
    fn 测试获取允许流转_验布() {
        let transitions = get_allowed_transitions("inspecting");
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&("stored", "store")));
        assert!(transitions.contains(&("rework", "rework")));
        assert!(transitions.contains(&("cancelled", "cancel")));
    }

    #[test]
    fn 测试获取允许流转_入库() {
        let transitions = get_allowed_transitions("stored");
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&("shipped", "ship")));
        assert!(transitions.contains(&("rework", "rework")));
        assert!(transitions.contains(&("cancelled", "cancel")));
    }

    #[test]
    fn 测试获取允许流转_回修中() {
        let transitions = get_allowed_transitions("rework");
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&("dyeing", "start_dyeing")));
        assert!(transitions.contains(&("cancelled", "cancel")));
        assert!(transitions.contains(&("terminated", "terminate")));
    }

    #[test]
    fn 测试获取允许流转_终态返回空() {
        assert!(get_allowed_transitions("shipped").is_empty());
        assert!(get_allowed_transitions("cancelled").is_empty());
        assert!(get_allowed_transitions("terminated").is_empty());
    }

    // ===== 流转校验（返回 Result）测试 =====

    #[test]
    fn test_transition_validation_valid_returns_ok() {
        assert!(
            validate_transition_with_rule(Some("pending_schedule"), "scheduled", "schedule")
                .is_ok()
        );
        assert!(validate_transition_with_rule(Some("dyeing"), "washing", "wash").is_ok());
        assert!(validate_transition_with_rule(Some("stored"), "shipped", "ship").is_ok());
    }

    #[test]
    fn test_transition_validation_invalid_returns_err() {
        assert!(
            validate_transition_with_rule(Some("pending_schedule"), "dyeing", "start_dyeing")
                .is_err()
        );
        assert!(validate_transition_with_rule(Some("shipped"), "stored", "store").is_err());
    }

    #[test]
    fn test_transition_validation_invalid_status_returns_err() {
        // to_status 非法
        assert!(
            validate_transition_with_rule(Some("pending_schedule"), "invalid", "schedule").is_err()
        );
        // transition_code 非法
        assert!(
            validate_transition_with_rule(Some("pending_schedule"), "scheduled", "invalid")
                .is_err()
        );
    }

    // ===== 回修资格校验测试 =====

    #[test]
    fn 测试回修资格_验布状态可回修() {
        assert!(check_rework_eligibility("inspecting").is_ok());
    }

    #[test]
    fn 测试回修资格_入库状态可回修() {
        assert!(check_rework_eligibility("stored").is_ok());
    }

    #[test]
    fn 测试回修资格_其他状态不可回修() {
        assert!(check_rework_eligibility("pending_schedule").is_err());
        assert!(check_rework_eligibility("scheduled").is_err());
        assert!(check_rework_eligibility("preparing").is_err());
        assert!(check_rework_eligibility("dyeing").is_err());
        assert!(check_rework_eligibility("washing").is_err());
        assert!(check_rework_eligibility("fixing").is_err());
        assert!(check_rework_eligibility("dehydrating").is_err());
        assert!(check_rework_eligibility("drying").is_err());
        assert!(check_rework_eligibility("shipped").is_err());
        assert!(check_rework_eligibility("cancelled").is_err());
        assert!(check_rework_eligibility("terminated").is_err());
        assert!(check_rework_eligibility("rework").is_err());
    }
}