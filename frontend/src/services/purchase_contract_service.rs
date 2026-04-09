//! 采购合同服务
//! 提供采购合同的增删改查等API调用

use crate::models::api_response::ApiResponse;
use crate::models::purchase_contract::{
    CancelPurchaseContractRequest, CreatePurchaseContractRequest, ExecutePurchaseContractRequest,
    PurchaseContract, PurchaseContractListResponse, PurchaseContractQueryParams,
};
use crate::services::api::ApiService;

/// 采购合同服务
pub struct PurchaseContractService;

impl PurchaseContractService {
    /// 获取采购合同列表
    pub async fn list_contracts(
        params: PurchaseContractQueryParams,
    ) -> Result<PurchaseContractListResponse, String> {
        let mut url = String::from("/purchase-contracts?");

        if let Some(keyword) = &params.keyword {
            url.push_str(&format!("keyword={}&", urlencoding::encode(keyword)));
        }
        if let Some(status) = &params.status {
            url.push_str(&format!("status={}&", status));
        }
        if let Some(supplier_id) = params.supplier_id {
            url.push_str(&format!("supplier_id={}&", supplier_id));
        }
        if let Some(page) = params.page {
            url.push_str(&format!("page={}&", page));
        }
        if let Some(page_size) = params.page_size {
            url.push_str(&format!("page_size={}", page_size));
        }

        let response: ApiResponse<PurchaseContractListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    /// 获取采购合同详情
    #[allow(dead_code)]
    pub async fn get_contract(id: i32) -> Result<PurchaseContract, String> {
        let response: ApiResponse<PurchaseContract> =
            ApiService::get(&format!("/purchase-contracts/{}", id)).await?;
        response.into_result()
    }

    /// 创建采购合同
    pub async fn create_contract(
        req: CreatePurchaseContractRequest,
    ) -> Result<PurchaseContract, String> {
        let response: ApiResponse<PurchaseContract> =
            ApiService::post("/purchase-contracts", &req).await?;
        response.into_result()
    }

    /// 审核采购合同
    pub async fn approve_contract(id: i32) -> Result<PurchaseContract, String> {
        let response: ApiResponse<PurchaseContract> = ApiService::put(
            &format!("/purchase-contracts/{}/approve", id),
            &serde_json::json!({}),
        )
        .await?;
        response.into_result()
    }

    /// 执行采购合同
    pub async fn execute_contract(
        id: i32,
        req: ExecutePurchaseContractRequest,
    ) -> Result<PurchaseContract, String> {
        let response: ApiResponse<PurchaseContract> =
            ApiService::put(&format!("/purchase-contracts/{}/execute", id), &req).await?;
        response.into_result()
    }

    /// 取消采购合同
    pub async fn cancel_contract(id: i32, reason: String) -> Result<PurchaseContract, String> {
        let req = CancelPurchaseContractRequest { reason };
        let response: ApiResponse<PurchaseContract> =
            ApiService::put(&format!("/purchase-contracts/{}/cancel", id), &req).await?;
        response.into_result()
    }
}
