//! 缸号全生命周期状态机 DTO
//!
//! 从 services/dye_batch_state_machine_service.rs 迁移的纯数据结构。
//! 包含生命周期日志、状态流转规则、回修记录、操作记录的请求与查询 DTO。

use serde::Deserialize;

// ============================================================================
// 缸号生命周期日志 DTO
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

// ============================================================================
// 缸号状态流转规则 DTO
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

// ============================================================================
// 缸号回修记录 DTO
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

// ============================================================================
// 缸号操作记录 DTO
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
