//! 销售合同服务
//! 提供销售合同的增删改查等API调用

use crate::models::api_response::ApiResponse;
use crate::models::sales_contract::{
    CancelSalesContractRequest, CreateSalesContractRequest, ExecuteSalesContractRequest,
    SalesContract, SalesContractListResponse, SalesContractQueryParams,
};
use crate::services::api::ApiService;

/// 销售合同服务
pub struct SalesContractService;

impl SalesContractService {
    /// 获取销售合同列表
    pub async fn list_contracts(
        params: SalesContractQueryParams,
    ) -> Result<SalesContractListResponse, String> {
        let mut url = String::from("/sales-contracts?");

        if let Some(keyword) = &params.keyword {
            url.push_str(&format!("keyword={}&", urlencoding::encode(keyword)));
        }
        if let Some(status) = &params.status {
            url.push_str(&format!("status={}&", status));
        }
        if let Some(customer_id) = params.customer_id {
            url.push_str(&format!("customer_id={}&", customer_id));
        }
        if let Some(page) = params.page {
            url.push_str(&format!("page={}&", page));
        }
        if let Some(page_size) = params.page_size {
            url.push_str(&format!("page_size={}", page_size));
        }

        let response: ApiResponse<SalesContractListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    /// 获取销售合同详情
    #[allow(dead_code)]
    pub async fn get_contract(id: i32) -> Result<SalesContract, String> {
        let response: ApiResponse<SalesContract> =
            ApiService::get(&format!("/sales-contracts/{}", id)).await?;
        response.into_result()
    }

    /// 创建销售合同
    pub async fn create_contract(req: CreateSalesContractRequest) -> Result<SalesContract, String> {
        let response: ApiResponse<SalesContract> =
            ApiService::post("/sales-contracts", &req).await?;
        response.into_result()
    }

    /// 审核销售合同
    pub async fn approve_contract(id: i32) -> Result<SalesContract, String> {
        let response: ApiResponse<SalesContract> = ApiService::put(
            &format!("/sales-contracts/{}/approve", id),
            &serde_json::json!({}),
        )
        .await?;
        response.into_result()
    }

    /// 执行销售合同
    pub async fn execute_contract(
        id: i32,
        req: ExecuteSalesContractRequest,
    ) -> Result<SalesContract, String> {
        let response: ApiResponse<SalesContract> =
            ApiService::put(&format!("/sales-contracts/{}/execute", id), &req).await?;
        response.into_result()
    }

    /// 取消销售合同
    pub async fn cancel_contract(id: i32, reason: String) -> Result<SalesContract, String> {
        let req = CancelSalesContractRequest { reason };
        let response: ApiResponse<SalesContract> =
            ApiService::put(&format!("/sales-contracts/{}/cancel", id), &req).await?;
        response.into_result()
    }
}
