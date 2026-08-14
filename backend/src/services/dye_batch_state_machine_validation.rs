//! 缸号状态机校验纯函数模块（dye_batch_state_machine_validation）
//!
//! V15 P2 B07-P2-1 修复：从原 `dye_batch_state_machine_service.rs` facade 拆出
//! 11 个纯验证函数 + 内置流转规则表 + 单元测试，降低 facade 行数（936→~510）。
//! 通过 facade 的 `pub use` 再导出，保持外部引用路径
//! `crate::services::dye_batch_state_machine_service::*` 不变。

use crate::models::status::dye_batch_lifecycle_status;
use crate::models::status::dye_batch_operation_type;
use crate::models::status::dye_batch_rework_status;
use crate::models::status::dye_batch_rework_type;
use crate::models::status::dye_batch_transition_code;
use crate::utils::error::AppError;

/// 校验缸号生命周期状态是否合法（16 种状态）
pub fn validate_lifecycle_status(status: &str) -> Result<(), AppError> {
    let valid = [
        dye_batch_lifecycle_status::PENDING_SCHEDULE,
        dye_batch_lifecycle_status::SCHEDULED,
        dye_batch_lifecycle_status::PREPARING,
        dye_batch_lifecycle_status::DYEING,
        dye_batch_lifecycle_status::WASHING,
        dye_batch_lifecycle_status::FIXING,
        dye_batch_lifecycle_status::DEHYDRATING,
        dye_batch_lifecycle_status::DRYING,
        dye_batch_lifecycle_status::INSPECTING,
        dye_batch_lifecycle_status::STORED,
        dye_batch_lifecycle_status::SHIPPED,
        dye_batch_lifecycle_status::CANCELLED,
        dye_batch_lifecycle_status::TERMINATED,
        dye_batch_lifecycle_status::REWORK,
        dye_batch_lifecycle_status::ON_HOLD,
        dye_batch_lifecycle_status::FAILED,
    ];
    if !valid.contains(&status) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "lifecycle_status_whitelist",
            invalid_status = status,
            "缸号状态机校验失败：生命周期状态不在白名单内"
        );
        return Err(AppError::business(format!(
            "缸号生命周期状态必须是 pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/shipped/cancelled/terminated/rework/on_hold/failed，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 校验缸号流转操作代码是否合法（16 种操作）
pub fn validate_transition_code(code: &str) -> Result<(), AppError> {
    let valid = [
        dye_batch_transition_code::SCHEDULE,
        dye_batch_transition_code::PREPARE,
        dye_batch_transition_code::START_DYEING,
        dye_batch_transition_code::WASH,
        dye_batch_transition_code::FIX,
        dye_batch_transition_code::DEHYDRATE,
        dye_batch_transition_code::DRY,
        dye_batch_transition_code::INSPECT,
        dye_batch_transition_code::STORE,
        dye_batch_transition_code::SHIP,
        dye_batch_transition_code::CANCEL,
        dye_batch_transition_code::REWORK,
        dye_batch_transition_code::TERMINATE,
        dye_batch_transition_code::HOLD,
        dye_batch_transition_code::RESUME,
        dye_batch_transition_code::FAIL,
    ];
    if !valid.contains(&code) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "transition_code_whitelist",
            invalid_code = code,
            "缸号状态机校验失败：流转操作代码不在白名单内"
        );
        return Err(AppError::business(format!(
            "缸号流转操作代码必须是 schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate/hold/resume/fail，当前: {}",
            code
        )));
    }
    Ok(())
}

/// 校验缸号回修类型是否合法（6 种类型，V15 P2 B05-P2-2 补 re_dye/replenish_dye）
pub fn validate_rework_type(rework_type: &str) -> Result<(), AppError> {
    let valid = [
        dye_batch_rework_type::COLOR_DIFFERENCE,
        dye_batch_rework_type::DEFECT,
        dye_batch_rework_type::SPECIFICATION_UNQUALIFIED,
        dye_batch_rework_type::RE_DYE,
        dye_batch_rework_type::REPLENISH_DYE,
        dye_batch_rework_type::OTHER,
    ];
    if !valid.contains(&rework_type) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "rework_type_whitelist",
            invalid_rework_type = rework_type,
            "缸号状态机校验失败：回修类型不在白名单内"
        );
        return Err(AppError::business(format!(
            "缸号回修类型必须是 color_difference/defect/specification_unqualified/re_dye/replenish_dye/other，当前: {}",
            rework_type
        )));
    }
    Ok(())
}

/// 校验缸号回修单状态是否合法（5 种状态）
pub fn validate_rework_status(status: &str) -> Result<(), AppError> {
    let valid = [
        dye_batch_rework_status::DRAFT,
        dye_batch_rework_status::APPROVED,
        dye_batch_rework_status::IN_PROGRESS,
        dye_batch_rework_status::COMPLETED,
        dye_batch_rework_status::CANCELLED,
    ];
    if !valid.contains(&status) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "rework_status_whitelist",
            invalid_status = status,
            "缸号状态机校验失败：回修单状态不在白名单内"
        );
        return Err(AppError::business(format!(
            "缸号回修单状态必须是 draft/approved/in_progress/completed/cancelled，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 校验缸号操作类型是否合法（6 种类型）
pub fn validate_operation_type(op_type: &str) -> Result<(), AppError> {
    let valid = [
        dye_batch_operation_type::MERGE,
        dye_batch_operation_type::SPLIT,
        dye_batch_operation_type::PRIORITY_ADJUST,
        dye_batch_operation_type::BATCH_CHANGE,
        dye_batch_operation_type::SCHEDULE_CHANGE,
        dye_batch_operation_type::TERMINATE,
    ];
    if !valid.contains(&op_type) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "operation_type_whitelist",
            invalid_op_type = op_type,
            "缸号状态机校验失败：操作类型不在白名单内"
        );
        return Err(AppError::business(format!(
            "缸号操作类型必须是 merge/split/priority_adjust/batch_change/schedule_change/terminate，当前: {}",
            op_type
        )));
    }
    Ok(())
}

/// 判断是否终态（shipped/cancelled/terminated/failed 不可再流转）
pub fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        dye_batch_lifecycle_status::SHIPPED
            | dye_batch_lifecycle_status::CANCELLED
            | dye_batch_lifecycle_status::TERMINATED
            | dye_batch_lifecycle_status::FAILED
    )
}

/// 内置流转规则表（与 SQL 预置数据 dye_batch_state_rule 一致）
fn builtin_transition_rules() -> Vec<(&'static str, &'static str, &'static str)> {
    use dye_batch_lifecycle_status::*;
    use dye_batch_transition_code::*;
    vec![
        // pending_schedule → scheduled / cancelled / failed
        (PENDING_SCHEDULE, SCHEDULED, SCHEDULE),
        (PENDING_SCHEDULE, CANCELLED, CANCEL),
        (PENDING_SCHEDULE, FAILED, FAIL),
        // scheduled → preparing / cancelled / terminated / on_hold / failed
        (SCHEDULED, PREPARING, PREPARE),
        (SCHEDULED, CANCELLED, CANCEL),
        (SCHEDULED, TERMINATED, TERMINATE),
        (SCHEDULED, ON_HOLD, HOLD),
        (SCHEDULED, FAILED, FAIL),
        // preparing → dyeing / cancelled / terminated / on_hold / failed
        (PREPARING, DYEING, START_DYEING),
        (PREPARING, CANCELLED, CANCEL),
        (PREPARING, TERMINATED, TERMINATE),
        (PREPARING, ON_HOLD, HOLD),
        (PREPARING, FAILED, FAIL),
        // dyeing → washing / cancelled / terminated / on_hold / failed
        (DYEING, WASHING, WASH),
        (DYEING, CANCELLED, CANCEL),
        (DYEING, TERMINATED, TERMINATE),
        (DYEING, ON_HOLD, HOLD),
        (DYEING, FAILED, FAIL),
        // washing → fixing / cancelled / on_hold / failed
        (WASHING, FIXING, FIX),
        (WASHING, CANCELLED, CANCEL),
        (WASHING, ON_HOLD, HOLD),
        (WASHING, FAILED, FAIL),
        // fixing → dehydrating / cancelled / on_hold / failed
        (FIXING, DEHYDRATING, DEHYDRATE),
        (FIXING, CANCELLED, CANCEL),
        (FIXING, ON_HOLD, HOLD),
        (FIXING, FAILED, FAIL),
        // dehydrating → drying / cancelled / on_hold / failed
        (DEHYDRATING, DRYING, DRY),
        (DEHYDRATING, CANCELLED, CANCEL),
        (DEHYDRATING, ON_HOLD, HOLD),
        (DEHYDRATING, FAILED, FAIL),
        // drying → inspecting / cancelled / on_hold / failed
        (DRYING, INSPECTING, INSPECT),
        (DRYING, CANCELLED, CANCEL),
        (DRYING, ON_HOLD, HOLD),
        (DRYING, FAILED, FAIL),
        // inspecting → stored / rework / cancelled / failed
        (INSPECTING, STORED, STORE),
        (
            INSPECTING,
            dye_batch_lifecycle_status::REWORK,
            dye_batch_transition_code::REWORK,
        ),
        (INSPECTING, CANCELLED, CANCEL),
        (INSPECTING, FAILED, FAIL),
        // stored → shipped / rework / cancelled / failed
        (STORED, SHIPPED, SHIP),
        (
            STORED,
            dye_batch_lifecycle_status::REWORK,
            dye_batch_transition_code::REWORK,
        ),
        (STORED, CANCELLED, CANCEL),
        (STORED, FAILED, FAIL),
        // rework → dyeing / cancelled / terminated / failed
        (dye_batch_lifecycle_status::REWORK, DYEING, START_DYEING),
        (dye_batch_lifecycle_status::REWORK, CANCELLED, CANCEL),
        (dye_batch_lifecycle_status::REWORK, TERMINATED, TERMINATE),
        (dye_batch_lifecycle_status::REWORK, FAILED, FAIL),
        // on_hold → 恢复到原工序（dyeing/washing/fixing/dehydrating/drying/scheduled/preparing）/ cancelled / failed
        // V15 Batch05-P1-1：on_hold 可恢复到染整各工序继续流转
        (ON_HOLD, DYEING, RESUME),
        (ON_HOLD, WASHING, RESUME),
        (ON_HOLD, FIXING, RESUME),
        (ON_HOLD, DEHYDRATING, RESUME),
        (ON_HOLD, DRYING, RESUME),
        (ON_HOLD, SCHEDULED, RESUME),
        (ON_HOLD, PREPARING, RESUME),
        (ON_HOLD, CANCELLED, CANCEL),
        (ON_HOLD, FAILED, FAIL),
    ]
}

/// 纯函数版状态流转校验（内置流转规则表）（from_status 为 None 表示初始状态（仅允许 pending_schedule → scheduled/cancelled））
pub fn is_valid_transition(
    from_status: Option<&str>,
    to_status: &str,
    transition_code: &str,
) -> bool {
    // 终态不可流转
    if let Some(from) = from_status {
        if is_terminal_status(from) {
            return false;
        }
    }
    // 校验 to_status 不是终态的来源时不能从终态过来（已在上面处理）
    let rules = builtin_transition_rules();
    rules.iter().any(|(from, to, code)| {
        match from_status {
            Some(fs) => fs == *from && to_status == *to && transition_code == *code,
            None => false, // from_status 为 None 时无匹配规则（初始状态由 pending_schedule 表示）
        }
    })
}

/// 获取指定状态允许的流转列表（to_status, transition_code）
pub fn get_allowed_transitions(from_status: &str) -> Vec<(&'static str, &'static str)> {
    if is_terminal_status(from_status) {
        return vec![];
    }
    let rules = builtin_transition_rules();
    rules
        .iter()
        .filter(|(from, _, _)| *from == from_status)
        .map(|(_, to, code)| (*to, *code))
        .collect()
}

/// 校验状态流转合法性（调用 is_valid_transition，失败返回业务错误）
pub fn validate_transition_with_rule(
    from_status: Option<&str>,
    to_status: &str,
    transition_code: &str,
) -> Result<(), AppError> {
    // 校验 to_status 合法
    validate_lifecycle_status(to_status)?;
    // 校验 transition_code 合法
    validate_transition_code(transition_code)?;
    // 校验 from_status 合法（若提供）
    if let Some(fs) = from_status {
        validate_lifecycle_status(fs)?;
    }
    if !is_valid_transition(from_status, to_status, transition_code) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "transition_rule_table",
            from_status = ?from_status,
            to_status = to_status,
            transition_code = transition_code,
            "缸号状态机校验失败：状态流转不在规则表内（非法流转）"
        );
        return Err(AppError::business(format!(
            "不允许的状态流转: {:?} → {}（操作代码: {}）",
            from_status, to_status, transition_code
        )));
    }
    Ok(())
}

/// 校验回修资格（只有 inspecting/stored 状态可回修）
pub fn check_rework_eligibility(original_status: &str) -> Result<(), AppError> {
    let eligible = [
        dye_batch_lifecycle_status::INSPECTING,
        dye_batch_lifecycle_status::STORED,
    ];
    if !eligible.contains(&original_status) {
        tracing::warn!(
            target: "dye_batch_state_machine",
            rule = "rework_eligibility",
            original_status = original_status,
            "缸号状态机校验失败：当前状态不可发起回修（仅 inspecting/stored 允许）"
        );
        return Err(AppError::business(format!(
            "只有 inspecting/stored 状态可发起回修，当前状态: {}",
            original_status
        )));
    }
    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================
