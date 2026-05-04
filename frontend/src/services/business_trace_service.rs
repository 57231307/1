use crate::models::api_response::ApiResponse;
use crate::models::business_trace::{
    FullTraceChainResponse, TraceListResponse,
};
use crate::services::api::ApiService;

/// 业务追溯服务
pub struct BusinessTraceService;

impl BusinessTraceService {
    /// 按五维ID查询追溯链
    #[allow(dead_code)]
    pub async fn get_trace_by_five_dimension(five_dimension_id: &str) -> Result<FullTraceChainResponse, String> {
        let response: FullTraceChainResponse = ApiService::get(&format!("/business-trace/five-dimension/{}", five_dimension_id)).await?;
        Ok(response)
    }

    /// 正向追溯 - 从供应商到最终客户
    #[allow(dead_code)]
    pub async fn forward_trace(supplier_id: i32, batch_no: &str) -> Result<TraceListResponse, String> {
        let response: TraceListResponse = ApiService::get(
            &format!("/business-trace/forward?supplier_id={}&batch_no={}", supplier_id, batch_no)
        ).await?;
        Ok(response)
    }

    /// 反向追溯 - 从客户追溯到供应商
    #[allow(dead_code)]
    pub async fn backward_trace(customer_id: i32, batch_no: &str) -> Result<TraceListResponse, String> {
        let response: TraceListResponse = ApiService::get(
            &format!("/business-trace/backward?customer_id={}&batch_no={}", customer_id, batch_no)
        ).await?;
        Ok(response)
    }

    /// 创建追溯快照
    #[allow(dead_code)]
    pub async fn create_snapshot(trace_chain_id: &str) -> Result<String, String> {
        let _response: ApiResponse<String> = ApiService::post(
            &format!("/business-trace/snapshot/{}", trace_chain_id),
            &serde_json::json!({})
        ).await?;
        Ok(format!("追溯快照 {} 创建成功", trace_chain_id))
    }
}
