use crate::services::api::ApiService;

/// 追溯链数据模型
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub warehouse_id: i32,
    pub supplier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub trace_status: String,
    pub created_at: String,
}

/// 追溯环节详情
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TraceStageDetail {
    pub stage_id: i32,
    pub stage_name: String,
    pub stage_type: String,
    pub bill_type: String,
    pub bill_no: String,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub supplier_name: Option<String>,
    pub customer_name: Option<String>,
    pub created_at: String,
}

/// 完整追溯链响应
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TraceListResponse {
    pub traces: Vec<TraceChain>,
    pub total: u64,
}

/// 正向追溯参数
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForwardTraceParams {
    pub supplier_id: i32,
    pub batch_no: String,
}

/// 反向追溯参数
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackwardTraceParams {
    pub customer_id: i32,
    pub batch_no: String,
}

/// 业务追溯服务
pub struct BusinessTraceService;

impl BusinessTraceService {
    /// 按五维ID查询追溯链
    #[allow(dead_code)]
    pub async fn get_trace_by_five_dimension(five_dimension_id: &str) -> Result<FullTraceChainResponse, String> {
        ApiService::get::<FullTraceChainResponse>(
            &format!("/api/v1/erp/business-trace/five-dimension/{}", five_dimension_id)
        ).await
    }

    /// 正向追溯 - 从供应商到最终客户
    #[allow(dead_code)]
    pub async fn forward_trace(supplier_id: i32, batch_no: &str) -> Result<TraceListResponse, String> {
        ApiService::get::<TraceListResponse>(
            &format!("/api/v1/erp/business-trace/forward?supplier_id={}&batch_no={}", supplier_id, batch_no)
        ).await
    }

    /// 反向追溯 - 从客户追溯到供应商
    #[allow(dead_code)]
    pub async fn backward_trace(customer_id: i32, batch_no: &str) -> Result<TraceListResponse, String> {
        ApiService::get::<TraceListResponse>(
            &format!("/api/v1/erp/business-trace/backward?customer_id={}&batch_no={}", customer_id, batch_no)
        ).await
    }

    /// 创建追溯快照
    #[allow(dead_code)]
    pub async fn create_snapshot(trace_chain_id: &str) -> Result<String, String> {
        let _: serde_json::Value = ApiService::post(
            &format!("/api/v1/erp/business-trace/snapshot/{}", trace_chain_id),
            &serde_json::json!({})
        ).await?;
        Ok(format!("追溯快照 {} 创建成功", trace_chain_id))
    }
}