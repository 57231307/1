//! 业务追溯模型

use serde::{Deserialize, Serialize};

/// 追溯链数据模型
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TraceChain {
    pub id: i32,
    pub trace_chain_id: String,
    pub five_dimension_id: String,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub current_stage: String,
    pub current_bill_type: String,
    pub current_bill_no: String,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub warehouse_id: i32,
    pub supplier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub trace_status: String,
    pub created_at: String,
}

/// 追溯环节详情
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TraceStageDetail {
    pub stage_id: i32,
    pub stage_name: String,
    pub stage_type: String,
    pub bill_type: String,
    pub bill_no: String,
    pub quantity_meters: String,
    pub quantity_kg: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub supplier_name: Option<String>,
    pub customer_name: Option<String>,
    pub created_at: String,
}

/// 完整追溯链响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FullTraceChainResponse {
    pub trace_chain_id: String,
    pub five_dimension_id: String,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub grade: String,
    pub stages: Vec<TraceStageDetail>,
    pub total_stages: usize,
    pub start_time: String,
    pub end_time: Option<String>,
}

/// 追溯列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct TraceListResponse {
    pub traces: Vec<TraceChain>,
    pub total: u64,
}

/// 正向追溯参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ForwardTraceParams {
    pub supplier_id: i32,
    pub batch_no: String,
}

/// 反向追溯参数
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct BackwardTraceParams {
    pub customer_id: i32,
    pub batch_no: String,
}
