use crate::models::fund_management::{
    CreateFundAccountRequest, FundAccount, FundAccountListResponse, FundAccountQueryParams,
};
use crate::services::api::ApiService;

/// 资金账户服务
pub struct FundManagementService;

impl FundManagementService {
    /// 获取资金账户列表
    pub async fn list_accounts(params: FundAccountQueryParams) -> Result<Vec<FundAccount>, String> {
        let mut url = "/fund-accounts".to_string();
        let mut query_parts: Vec<String> = Vec::new();

        if let Some(account_type) = &params.account_type {
            query_parts.push(format!("account_type={}", account_type));
        }
        if let Some(status) = &params.status {
            query_parts.push(format!("status={}", status));
        }
        if let Some(page) = params.page {
            query_parts.push(format!("page={}", page));
        }
        if let Some(page_size) = params.page_size {
            query_parts.push(format!("page_size={}", page_size));
        }

        if !query_parts.is_empty() {
            url.push_str(&format!("?{}", query_parts.join("&")));
        }

        let response: FundAccountListResponse = ApiService::get(&url).await?;
        Ok(response.data)
    }

    /// 获取资金账户详情
    pub async fn get_account(id: i32) -> Result<FundAccount, String> {
        ApiService::get::<FundAccount>(&format!("/fund-accounts/{}", id)).await
    }

    /// 创建资金账户
    pub async fn create_account(req: CreateFundAccountRequest) -> Result<FundAccount, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/fund-accounts", &payload).await
    }

    /// 账户存款
    pub async fn deposit(id: i32, amount: String, remark: Option<String>) -> Result<String, String> {
        #[derive(Debug, Clone, serde::Serialize)]
        struct DepositRequest {
            amount: String,
            remark: Option<String>,
        }
        let req = DepositRequest { amount, remark };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/fund-accounts/{}/deposit", id), &payload).await?;
        Ok("存款成功".to_string())
    }

    /// 账户取款
    pub async fn withdraw(id: i32, amount: String, remark: Option<String>) -> Result<String, String> {
        #[derive(Debug, Clone, serde::Serialize)]
        struct WithdrawRequest {
            amount: String,
            remark: Option<String>,
        }
        let req = WithdrawRequest { amount, remark };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/fund-accounts/{}/withdraw", id), &payload).await?;
        Ok("取款成功".to_string())
    }

    /// 冻结账户资金
    pub async fn freeze_funds(id: i32, amount: String, reason: String) -> Result<String, String> {
        #[derive(Debug, Clone, serde::Serialize)]
        struct FreezeRequest {
            amount: String,
            reason: String,
        }
        let req = FreezeRequest { amount, reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/fund-accounts/{}/freeze", id), &payload).await?;
        Ok("冻结成功".to_string())
    }

    /// 解冻账户资金
    pub async fn unfreeze_funds(id: i32, amount: String, remark: Option<String>) -> Result<String, String> {
        #[derive(Debug, Clone, serde::Serialize)]
        struct UnfreezeRequest {
            amount: String,
            remark: Option<String>,
        }
        let req = UnfreezeRequest { amount, remark };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        let _: serde_json::Value = ApiService::post(&format!("/fund-accounts/{}/unfreeze", id), &payload).await?;
        Ok("解冻成功".to_string())
    }

    /// 删除资金账户
    pub async fn delete_account(id: i32) -> Result<String, String> {
        ApiService::delete(&format!("/fund-accounts/{}", id))
            .await
            .map(|_| "删除成功".to_string())
    }

    /// 资金调拨
    pub async fn transfer_fund(from_account_id: i32, to_account_id: i32, amount: String, fee: Option<String>, reason: Option<String>) -> Result<serde_json::Value, String> {
        #[derive(Debug, Clone, serde::Serialize)]
        struct TransferRequest {
            from_account_id: i32,
            to_account_id: i32,
            amount: String,
            fee: Option<String>,
            reason: Option<String>,
        }
        let req = TransferRequest { from_account_id, to_account_id, amount, fee, reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/fund-management/transfer", &payload).await
    }
}