use crate::services::api::ApiService;

/// 客户信用数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CustomerCredit {
    pub id: i32,
    pub customer_id: i32,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: Option<f64>,
    pub used_credit: Option<f64>,
    pub available_credit: Option<f64>,
    pub credit_days: Option<i32>,
    pub status: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 客户信用列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CustomerCreditListResponse {
    pub data: Vec<CustomerCredit>,
    pub total: Option<u64>,
}

/// 客户信用查询参数
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreditQueryParams {
    pub customer_id: Option<i32>,
    pub credit_level: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 信用评级设置请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreditRatingRequest {
    pub customer_id: i32,
    pub credit_level: String,
    pub credit_score: i32,
    pub credit_limit: f64,
    pub credit_days: i32,
    pub remark: Option<String>,
}

/// 信用额度调整请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreditLimitAdjustmentRequest {
    pub adjustment_type: String,
    pub amount: f64,
    pub reason: String,
}

/// 信用额度占用/释放请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreditAmountRequest {
    pub amount: f64,
}

/// 客户信用服务
pub struct CustomerCreditService;

impl CustomerCreditService {
    /// 获取客户信用列表
    pub async fn list_credits(params: CreditQueryParams) -> Result<CustomerCreditListResponse, String> {
        let mut url = String::from("/api/v1/erp/customer-credits?");
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
        ApiService::get::<CustomerCreditListResponse>(&url).await
    }

    /// 获取客户信用详情
    pub async fn get_credit(customer_id: i32) -> Result<CustomerCredit, String> {
        ApiService::get::<CustomerCredit>(&format!("/api/v1/erp/customer-credits/{}", customer_id)).await
    }

    /// 设置客户信用评级
    pub async fn set_credit_rating(req: CreditRatingRequest) -> Result<CustomerCredit, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/customer-credits/rating", &payload).await
    }

    /// 占用信用额度
    pub async fn occupy_credit(customer_id: i32, amount: f64) -> Result<String, String> {
        let req = CreditAmountRequest { amount };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/api/v1/erp/customer-credits/{}/occupy", customer_id), &payload).await?;
        Ok(format!("客户 {} 信用额度占用成功", customer_id))
    }

    /// 释放信用额度
    pub async fn release_credit(customer_id: i32, amount: f64) -> Result<String, String> {
        let req = CreditAmountRequest { amount };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/api/v1/erp/customer-credits/{}/release", customer_id), &payload).await?;
        Ok(format!("客户 {} 信用额度释放成功", customer_id))
    }

    /// 调整信用额度
    pub async fn adjust_credit_limit(customer_id: i32, req: CreditLimitAdjustmentRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/api/v1/erp/customer-credits/{}/adjust", customer_id), &payload).await?;
        Ok(format!("客户 {} 信用额度调整成功", customer_id))
    }

    /// 停用客户信用
    pub async fn deactivate_credit(customer_id: i32) -> Result<String, String> {
        let _: serde_json::Value = ApiService::post(&format!("/api/v1/erp/customer-credits/{}/deactivate", customer_id), &serde_json::json!({})).await?;
        Ok(format!("客户 {} 信用停用成功", customer_id))
    }
}