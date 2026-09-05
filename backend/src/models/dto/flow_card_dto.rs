//! 流转卡与工序流转 DTO
//!
//! 从 services/flow_card_service.rs 迁移的纯数据结构。
//! 包含工序路线、流转卡、工序流转记录、质量反馈单的请求与查询 DTO。

use rust_decimal::Decimal;
use serde::Deserialize;

// ============================================================================
// 工序路线模板 DTO
// ============================================================================

/// 创建工序路线请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateProcessRouteRequest {
    pub route_code: String,
    pub route_name: String,
    pub seq: i32,
    pub process_type: String,
    pub default_duration_minutes: Option<i32>,
    pub require_scan: Option<bool>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工序路线请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProcessRouteRequest {
    pub route_name: Option<String>,
    pub seq: Option<i32>,
    pub process_type: Option<String>,
    pub default_duration_minutes: Option<i32>,
    pub require_scan: Option<bool>,
    pub is_active: Option<bool>,
    pub remarks: Option<String>,
}

// ============================================================================
// 流转卡 DTO
// ============================================================================

/// 创建流转卡请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFlowCardRequest {
    pub production_order_id: i32,
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub dyeing_requirements: Option<String>,
    pub planned_fabric_weight: Option<Decimal>,
    pub priority: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新流转卡请求（仅 pending 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFlowCardRequest {
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub process_route_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub order_no: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub color_no: Option<String>,
    pub dyeing_requirements: Option<String>,
    pub planned_fabric_weight: Option<Decimal>,
    pub priority: Option<i32>,
    pub remarks: Option<String>,
}

/// 流转卡查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct FlowCardQuery {
    pub card_no: Option<String>,
    pub barcode: Option<String>,
    pub dye_lot_no: Option<String>,
    pub production_order_id: Option<i32>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 工序流转记录 DTO
// ============================================================================

/// 开始工序请求（扫码开始）
#[derive(Debug, Clone, Deserialize)]
pub struct StartStepRequest {
    pub flow_card_id: i32,
    pub process_route_id: Option<i32>,
    pub worker_ids: Option<String>,
    pub worker_names: Option<String>,
    pub equipment_id: Option<i32>,
    pub equipment_name: Option<String>,
    pub created_by: Option<i32>,
}

/// 结束工序请求（扫码结束）
#[derive(Debug, Clone, Deserialize)]
pub struct CompleteStepRequest {
    pub actual_quantity: Option<Decimal>,
    pub qualified_quantity: Option<Decimal>,
    pub abnormal_description: Option<String>,
    pub handling_opinion: Option<String>,
    pub remarks: Option<String>,
    /// 生产报工逐匹登记（匹号领域：胚布产出必须逐匹登记生产匹号+机台号+开机人）。
    /// 传入时为工艺单产出创建生产匹（piece_type=greige），仓库必须为胚布仓
    #[serde(default)]
    pub pieces: Option<Vec<crate::services::piece_domain_service::ReportPieceInput>>,
}

// ============================================================================
// 工序质量反馈单 DTO
// ============================================================================

/// 创建质量反馈单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFeedbackRequest {
    pub flow_card_id: i32,
    pub step_record_id: Option<i32>,
    pub feedback_type: String,
    pub description: String,
    pub severity: Option<String>,
    pub found_by: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 处理反馈单请求
#[derive(Debug, Clone, Deserialize)]
pub struct HandleFeedbackRequest {
    pub handling_opinion: Option<String>,
    pub handling_result: Option<String>,
    pub handled_by: Option<i32>,
}
