//! 缸号全生命周期状态机 Service（facade）
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
//!
//! 核心能力：缸号生命周期日志 CRUD + 按 batch_id 查询 + 按时间范围查询 + 记录状态流转 + 获取最新状态；
//! 缸号状态规则 CRUD + 校验流转合法性 + 查询允许的流转；
//! 缸号回修记录 CRUD + 审批 + 开始回修 + 完成回修 + 取消回修；
//! 缸号操作记录 CRUD + 按类型查询 + 按缸号查询。
//!
//! 14 种状态：pending_schedule 待排缸 / scheduled 已排缸 / preparing 备布中 / dyeing 进缸染色 / washing 皂洗 / fixing 固色 / dehydrating 脱水 / drying 烘干 / inspecting 验布 / stored 入库 / shipped 发货（终态）/ cancelled 取消（终态）/ terminated 终止（终态）/ rework 回修中（可回到 dyeing）。
//!
//! 批次 490 D10-4a 拆分：本文件作为 facade，保留 4 个 Service struct + new 构造函数
//! + 10 个 DTOs + 11 个纯验证函数 + 单元测试。4 个 Service 的业务方法 impl 块
//! 迁移至 `dye_batch_state_machine_ops` 子模块（lifecycle_log / state_rule / rework / operation），
//! 通过跨模块 `impl XxxService` 追加方法，保持外部引用路径
//! `crate::services::dye_batch_state_machine_service::*` 不变。

use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use crate::models::status::dye_batch_lifecycle_status;
use crate::models::status::dye_batch_operation_type;
use crate::models::status::dye_batch_rework_status;
use crate::models::status::dye_batch_rework_type;
use crate::models::status::dye_batch_transition_code;
use crate::utils::error::AppError;

// ============================================================================
// 缸号状态机校验纯函数
// ============================================================================

/// 校验缸号生命周期状态是否合法（14 种状态）
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
    ];
    if !valid.contains(&status) {
        return Err(AppError::business(format!(
            "缸号生命周期状态必须是 pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/shipped/cancelled/terminated/rework，当前: {}",
            status
        )));
    }
    Ok(())
}

/// 校验缸号流转操作代码是否合法（13 种操作）
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
    ];
    if !valid.contains(&code) {
        return Err(AppError::business(format!(
            "缸号流转操作代码必须是 schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate，当前: {}",
            code
        )));
    }
    Ok(())
}

/// 校验缸号回修类型是否合法（4 种类型）
pub fn validate_rework_type(rework_type: &str) -> Result<(), AppError> {
    let valid = [
        dye_batch_rework_type::COLOR_DIFFERENCE,
        dye_batch_rework_type::DEFECT,
        dye_batch_rework_type::SPECIFICATION_UNQUALIFIED,
        dye_batch_rework_type::OTHER,
    ];
    if !valid.contains(&rework_type) {
        return Err(AppError::business(format!(
            "缸号回修类型必须是 color_difference/defect/specification_unqualified/other，当前: {}",
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
        return Err(AppError::business(format!(
            "缸号操作类型必须是 merge/split/priority_adjust/batch_change/schedule_change/terminate，当前: {}",
            op_type
        )));
    }
    Ok(())
}

/// 判断是否终态（shipped/cancelled/terminated 不可再流转）
pub fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        dye_batch_lifecycle_status::SHIPPED
            | dye_batch_lifecycle_status::CANCELLED
            | dye_batch_lifecycle_status::TERMINATED
    )
}

/// 内置流转规则表（与 SQL 预置数据 dye_batch_state_rule 一致）
///
/// 返回 (from_status, to_status, transition_code) 三元组列表
fn builtin_transition_rules() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // pending_schedule → scheduled / cancelled
        (
            dye_batch_lifecycle_status::PENDING_SCHEDULE,
            dye_batch_lifecycle_status::SCHEDULED,
            dye_batch_transition_code::SCHEDULE,
        ),
        (
            dye_batch_lifecycle_status::PENDING_SCHEDULE,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // scheduled → preparing / cancelled / terminated
        (
            dye_batch_lifecycle_status::SCHEDULED,
            dye_batch_lifecycle_status::PREPARING,
            dye_batch_transition_code::PREPARE,
        ),
        (
            dye_batch_lifecycle_status::SCHEDULED,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        (
            dye_batch_lifecycle_status::SCHEDULED,
            dye_batch_lifecycle_status::TERMINATED,
            dye_batch_transition_code::TERMINATE,
        ),
        // preparing → dyeing / cancelled / terminated
        (
            dye_batch_lifecycle_status::PREPARING,
            dye_batch_lifecycle_status::DYEING,
            dye_batch_transition_code::START_DYEING,
        ),
        (
            dye_batch_lifecycle_status::PREPARING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        (
            dye_batch_lifecycle_status::PREPARING,
            dye_batch_lifecycle_status::TERMINATED,
            dye_batch_transition_code::TERMINATE,
        ),
        // dyeing → washing / cancelled / terminated
        (
            dye_batch_lifecycle_status::DYEING,
            dye_batch_lifecycle_status::WASHING,
            dye_batch_transition_code::WASH,
        ),
        (
            dye_batch_lifecycle_status::DYEING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        (
            dye_batch_lifecycle_status::DYEING,
            dye_batch_lifecycle_status::TERMINATED,
            dye_batch_transition_code::TERMINATE,
        ),
        // washing → fixing / cancelled
        (
            dye_batch_lifecycle_status::WASHING,
            dye_batch_lifecycle_status::FIXING,
            dye_batch_transition_code::FIX,
        ),
        (
            dye_batch_lifecycle_status::WASHING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // fixing → dehydrating / cancelled
        (
            dye_batch_lifecycle_status::FIXING,
            dye_batch_lifecycle_status::DEHYDRATING,
            dye_batch_transition_code::DEHYDRATE,
        ),
        (
            dye_batch_lifecycle_status::FIXING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // dehydrating → drying / cancelled
        (
            dye_batch_lifecycle_status::DEHYDRATING,
            dye_batch_lifecycle_status::DRYING,
            dye_batch_transition_code::DRY,
        ),
        (
            dye_batch_lifecycle_status::DEHYDRATING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // drying → inspecting / cancelled
        (
            dye_batch_lifecycle_status::DRYING,
            dye_batch_lifecycle_status::INSPECTING,
            dye_batch_transition_code::INSPECT,
        ),
        (
            dye_batch_lifecycle_status::DRYING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // inspecting → stored / rework / cancelled
        (
            dye_batch_lifecycle_status::INSPECTING,
            dye_batch_lifecycle_status::STORED,
            dye_batch_transition_code::STORE,
        ),
        (
            dye_batch_lifecycle_status::INSPECTING,
            dye_batch_lifecycle_status::REWORK,
            dye_batch_transition_code::REWORK,
        ),
        (
            dye_batch_lifecycle_status::INSPECTING,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // stored → shipped / rework / cancelled
        (
            dye_batch_lifecycle_status::STORED,
            dye_batch_lifecycle_status::SHIPPED,
            dye_batch_transition_code::SHIP,
        ),
        (
            dye_batch_lifecycle_status::STORED,
            dye_batch_lifecycle_status::REWORK,
            dye_batch_transition_code::REWORK,
        ),
        (
            dye_batch_lifecycle_status::STORED,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        // rework → dyeing / cancelled / terminated
        (
            dye_batch_lifecycle_status::REWORK,
            dye_batch_lifecycle_status::DYEING,
            dye_batch_transition_code::START_DYEING,
        ),
        (
            dye_batch_lifecycle_status::REWORK,
            dye_batch_lifecycle_status::CANCELLED,
            dye_batch_transition_code::CANCEL,
        ),
        (
            dye_batch_lifecycle_status::REWORK,
            dye_batch_lifecycle_status::TERMINATED,
            dye_batch_transition_code::TERMINATE,
        ),
    ]
}

/// 纯函数版状态流转校验（内置流转规则表）
///
/// from_status 为 None 表示初始状态（仅允许 pending_schedule → scheduled/cancelled）
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
        return Err(AppError::business(format!(
            "只有 inspecting/stored 状态可发起回修，当前状态: {}",
            original_status
        )));
    }
    Ok(())
}

// ============================================================================
// 缸号生命周期日志 Service
// ============================================================================

/// 记录状态流转请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTransitionRequest {
    pub batch_id: i32,
    pub batch_no: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub transition_code: String,
    pub transition_name: String,
    pub operator_id: Option<i32>,
    pub operator_name: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub work_shift: Option<String>,
    pub captured_params: Option<serde_json::Value>,
    pub remarks: Option<String>,
}

/// 生命周期日志查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct LifecycleLogQuery {
    pub batch_id: Option<i32>,
    pub batch_no: Option<String>,
    pub transition_code: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缸号生命周期日志 Service
///
/// 业务方法（record_transition / get_by_id / list_by_batch / list_by_batch_no
/// / get_latest_status / list）定义在 dye_batch_state_machine_ops::lifecycle_log。
pub struct DyeBatchLifecycleLogService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchLifecycleLogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 缸号状态流转规则 Service
// ============================================================================

/// 创建状态流转规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateStateRuleRequest {
    pub from_status: String,
    pub to_status: String,
    pub transition_code: String,
    pub transition_name: String,
    pub is_allowed: Option<bool>,
    pub require_operator: Option<bool>,
    pub require_equipment: Option<bool>,
    pub require_remarks: Option<bool>,
    pub validation_logic: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// 更新状态流转规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStateRuleRequest {
    pub transition_name: Option<String>,
    pub is_allowed: Option<bool>,
    pub require_operator: Option<bool>,
    pub require_equipment: Option<bool>,
    pub require_remarks: Option<bool>,
    pub validation_logic: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// 状态规则查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct StateRuleQuery {
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub transition_code: Option<String>,
    pub is_active: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缸号状态流转规则 Service
///
/// 业务方法（create / update / delete / get_by_id / check_transition
/// / list_allowed_transitions / list）定义在 dye_batch_state_machine_ops::state_rule。
pub struct DyeBatchStateRuleService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchStateRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 缸号回修记录 Service
// ============================================================================

/// 创建回修记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateReworkRequest {
    pub original_batch_id: i32,
    pub original_batch_no: String,
    pub rework_batch_id: Option<i32>,
    pub rework_batch_no: Option<String>,
    pub rework_type: String,
    pub rework_reason: String,
    pub original_status: String,
    pub remarks: Option<String>,
}

/// 更新回修记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateReworkRequest {
    pub rework_type: Option<String>,
    pub rework_reason: Option<String>,
    pub rework_batch_id: Option<i32>,
    pub rework_batch_no: Option<String>,
    pub remarks: Option<String>,
}

/// 回修记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ReworkQuery {
    pub original_batch_id: Option<i32>,
    pub rework_batch_id: Option<i32>,
    pub rework_type: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缸号回修记录 Service
///
/// 业务方法（create / update / delete / get_by_id / approve / start_rework
/// / complete_rework / cancel_rework / list）定义在 dye_batch_state_machine_ops::rework。
pub struct DyeBatchReworkService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchReworkService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 缸号操作记录 Service
// ============================================================================

/// 创建操作记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOperationRequest {
    pub operation_type: String,
    pub operation_name: String,
    pub target_batch_id: i32,
    pub target_batch_no: String,
    pub source_batch_ids: Option<serde_json::Value>,
    pub source_batch_nos: Option<serde_json::Value>,
    pub operation_data: Option<serde_json::Value>,
    pub operator_id: Option<i32>,
    pub operator_name: Option<String>,
    pub remarks: Option<String>,
}

/// 操作记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OperationQuery {
    pub operation_type: Option<String>,
    pub target_batch_id: Option<i32>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缸号操作记录 Service
///
/// 业务方法（create / get_by_id / list_by_type / list_by_batch / list）
/// 定义在 dye_batch_state_machine_ops::operation。
pub struct DyeBatchOperationService {
    /// 数据库连接（pub(crate) 供 dye_batch_state_machine_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl DyeBatchOperationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(is_valid_transition(Some("pending_schedule"), "scheduled", "schedule"));
        // scheduled → preparing（备布）
        assert!(is_valid_transition(Some("scheduled"), "preparing", "prepare"));
        // preparing → dyeing（进缸染色）
        assert!(is_valid_transition(Some("preparing"), "dyeing", "start_dyeing"));
        // dyeing → washing（皂洗）
        assert!(is_valid_transition(Some("dyeing"), "washing", "wash"));
        // washing → fixing（固色）
        assert!(is_valid_transition(Some("washing"), "fixing", "fix"));
        // fixing → dehydrating（脱水）
        assert!(is_valid_transition(Some("fixing"), "dehydrating", "dehydrate"));
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
        assert!(is_valid_transition(Some("rework"), "dyeing", "start_dyeing"));
    }

    #[test]
    fn 测试状态流转_取消流转合法() {
        // 任意非终态 → cancelled
        assert!(is_valid_transition(Some("pending_schedule"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("scheduled"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("preparing"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("dyeing"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("washing"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("fixing"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("dehydrating"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("drying"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("inspecting"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("stored"), "cancelled", "cancel"));
        assert!(is_valid_transition(Some("rework"), "cancelled", "cancel"));
    }

    #[test]
    fn 测试状态流转_终止流转合法() {
        // scheduled/preparing/dyeing/rework → terminated
        assert!(is_valid_transition(Some("scheduled"), "terminated", "terminate"));
        assert!(is_valid_transition(Some("preparing"), "terminated", "terminate"));
        assert!(is_valid_transition(Some("dyeing"), "terminated", "terminate"));
        assert!(is_valid_transition(Some("rework"), "terminated", "terminate"));
    }

    #[test]
    fn 测试状态流转_终态不可流转() {
        // shipped 不可流转
        assert!(!is_valid_transition(Some("shipped"), "stored", "store"));
        assert!(!is_valid_transition(Some("shipped"), "cancelled", "cancel"));
        // cancelled 不可流转
        assert!(!is_valid_transition(Some("cancelled"), "scheduled", "schedule"));
        assert!(!is_valid_transition(Some("cancelled"), "terminated", "terminate"));
        // terminated 不可流转
        assert!(!is_valid_transition(Some("terminated"), "scheduled", "schedule"));
        assert!(!is_valid_transition(Some("terminated"), "cancelled", "cancel"));
    }

    #[test]
    fn 测试状态流转_非法流转() {
        // pending_schedule 不能直接到 dyeing
        assert!(!is_valid_transition(Some("pending_schedule"), "dyeing", "start_dyeing"));
        // scheduled 不能直接到 washing
        assert!(!is_valid_transition(Some("scheduled"), "washing", "wash"));
        // dyeing 不能直接到 inspecting（必须经过 washing/fixing/dehydrating/drying）
        assert!(!is_valid_transition(Some("dyeing"), "inspecting", "inspect"));
        // inspecting 不能直接到 shipped（必须经过 stored）
        assert!(!is_valid_transition(Some("inspecting"), "shipped", "ship"));
        // 操作代码不匹配
        assert!(!is_valid_transition(Some("pending_schedule"), "scheduled", "prepare"));
    }

    #[test]
    fn 测试状态流转_from_status为None返回false() {
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
    fn 测试流转校验_合法返回Ok() {
        assert!(validate_transition_with_rule(Some("pending_schedule"), "scheduled", "schedule").is_ok());
        assert!(validate_transition_with_rule(Some("dyeing"), "washing", "wash").is_ok());
        assert!(validate_transition_with_rule(Some("stored"), "shipped", "ship").is_ok());
    }

    #[test]
    fn 测试流转校验_非法返回Err() {
        assert!(validate_transition_with_rule(Some("pending_schedule"), "dyeing", "start_dyeing").is_err());
        assert!(validate_transition_with_rule(Some("shipped"), "stored", "store").is_err());
    }

    #[test]
    fn 测试流转校验_非法状态返回Err() {
        // to_status 非法
        assert!(validate_transition_with_rule(Some("pending_schedule"), "invalid", "schedule").is_err());
        // transition_code 非法
        assert!(validate_transition_with_rule(Some("pending_schedule"), "scheduled", "invalid").is_err());
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
