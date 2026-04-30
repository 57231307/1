use crate::models::api_response::ApiResponse;
use crate::models::customer_credit::{
    CreditAmountRequest, CreditLimitAdjustmentRequest, CreditQueryParams, CreditRatingRequest,
    CustomerCredit, CustomerCreditListResponse,
};
use crate::services::api::ApiService;

/// 客户信用服务
pub struct CustomerCreditService;

impl CustomerCreditService {
    /// 获取客户信用列表
    pub async fn list_credits(
        params: CreditQueryParams,
    ) -> Result<CustomerCreditListResponse, String> {
        let mut url = String::from("/customer-credits?");
        if let Some(customer_id) = params.customer_id {
            url.push_str(&format!("customer_id={}&", customer_id));
        }
        if let Some(credit_level) = &params.credit_level {
            url.push_str(&format!("credit_level={}&", credit_level));
        }
        if let Some(status) = &params.status {
            url.push_str(&format!("status={}&", status));
        }
        if let Some(page) = params.page {
            url.push_str(&format!("page={}&", page));
        }
        if let Some(page_size) = params.page_size {
            url.push_str(&format!("page_size={}", page_size));
        }
        let response: ApiResponse<CustomerCreditListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    /// 获取客户信用详情
    pub async fn get_credit(customer_id: i32) -> Result<CustomerCredit, String> {
        let response: ApiResponse<CustomerCredit> =
            ApiService::get(&format!("/customer-credits/{}", customer_id)).await?;
        response.into_result()
    }

    /// 设置客户信用评级
    pub async fn set_credit_rating(req: CreditRatingRequest) -> Result<CustomerCredit, String> {
        let response: ApiResponse<CustomerCredit> =
            ApiService::post("/customer-credits/rating", &req).await?;
        response.into_result()
    }

    /// 占用信用额度
    pub async fn occupy_credit(customer_id: i32, amount: String) -> Result<CustomerCredit, String> {
        let req = CreditAmountRequest { amount };
        let response: ApiResponse<CustomerCredit> =
            ApiService::post(&format!("/customer-credits/{}/occupy", customer_id), &req).await?;
        response.into_result()
    }

    /// 释放信用额度
    pub async fn release_credit(
        customer_id: i32,
        amount: String,
    ) -> Result<CustomerCredit, String> {
        let req = CreditAmountRequest { amount };
        let response: ApiResponse<CustomerCredit> =
            ApiService::post(&format!("/customer-credits/{}/release", customer_id), &req).await?;
        response.into_result()
    }

    /// 调整信用额度
    pub async fn adjust_credit_limit(
        customer_id: i32,
        req: CreditLimitAdjustmentRequest,
    ) -> Result<CustomerCredit, String> {
        let response: ApiResponse<CustomerCredit> =
            ApiService::post(&format!("/customer-credits/{}/adjust", customer_id), &req).await?;
        response.into_result()
    }

    /// 停用客户信用
    pub async fn deactivate_credit(customer_id: i32) -> Result<CustomerCredit, String> {
        let response: ApiResponse<CustomerCredit> = ApiService::post(
            &format!("/customer-credits/{}/deactivate", customer_id),
            &serde_json::json!({}),
        )
        .await?;
        response.into_result()
    }
}
