//! 多业务模式 DTO 子模块（business_mode_ops/types）
//!
//! 批次 489 D10-2b 拆分：从原 `business_mode_service.rs` 迁移 10 个 DTO struct。
//! 包含业务模式配置/流程节点/规则/单据关联的 Create/Update/Query 请求体。

use serde::Deserialize;

// ============================================================================
// 业务模式配置 DTO
// ============================================================================

/// 创建业务模式配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeConfigRequest {
    pub mode_code: String,
    pub mode_name: String,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub process_chain: Option<serde_json::Value>,
    pub material_source: String,
    pub settlement_method: String,
    pub inventory_type: String,
    pub cost_method: String,
    pub require_purchase: Option<bool>,
    pub require_production: Option<bool>,
    pub require_outsourcing: Option<bool>,
    pub require_sales: Option<bool>,
    pub mode_category: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新业务模式配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeConfigRequest {
    pub mode_name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub process_chain: Option<serde_json::Value>,
    pub material_source: Option<String>,
    pub settlement_method: Option<String>,
    pub inventory_type: Option<String>,
    pub cost_method: Option<String>,
    pub require_purchase: Option<bool>,
    pub require_production: Option<bool>,
    pub require_outsourcing: Option<bool>,
    pub require_sales: Option<bool>,
    pub mode_category: Option<String>,
    pub remarks: Option<String>,
}

/// 业务模式配置查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct BusinessModeConfigQuery {
    pub mode_code: Option<String>,
    pub mode_category: Option<String>,
    pub is_active: Option<bool>,
    pub material_source: Option<String>,
    pub settlement_method: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 业务模式流程节点 DTO
// ============================================================================

/// 创建业务模式流程节点请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeFlowStepRequest {
    pub mode_id: i32,
    pub step_no: i32,
    pub step_code: String,
    pub step_name: String,
    pub module_name: String,
    pub is_required: Option<bool>,
    pub description: Option<String>,
}

/// 更新业务模式流程节点请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeFlowStepRequest {
    pub step_no: Option<i32>,
    pub step_code: Option<String>,
    pub step_name: Option<String>,
    pub module_name: Option<String>,
    pub is_required: Option<bool>,
    pub description: Option<String>,
}

// ============================================================================
// 业务模式规则 DTO
// ============================================================================

/// 创建业务模式规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeRuleRequest {
    pub mode_id: i32,
    pub rule_code: String,
    pub rule_name: String,
    pub rule_type: String,
    pub module_name: String,
    pub validation_logic: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// 更新业务模式规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeRuleRequest {
    pub rule_code: Option<String>,
    pub rule_name: Option<String>,
    pub rule_type: Option<String>,
    pub module_name: Option<String>,
    pub validation_logic: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================================================
// 单据-业务模式关联 DTO
// ============================================================================

/// 创建单据-业务模式关联请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeOrderLinkRequest {
    pub mode_id: i32,
    pub document_type: String,
    pub document_id: i32,
    pub document_no: String,
    pub mode_snapshot: Option<serde_json::Value>,
    pub remarks: Option<String>,
}

/// 更新单据-业务模式关联请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeOrderLinkRequest {
    pub mode_id: Option<i32>,
    pub mode_snapshot: Option<serde_json::Value>,
    pub remarks: Option<String>,
}

/// 单据-业务模式关联查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct BusinessModeOrderLinkQuery {
    pub mode_id: Option<i32>,
    pub document_type: Option<String>,
    pub document_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
