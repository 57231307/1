//! 销售合同服务
//! 提供销售合同的增删改查等API调用

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 销售合同数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SalesContract {
    /// 合同ID
    pub id: i32,
    /// 合同编号
    pub contract_no: String,
    /// 合同名称
    pub contract_name: String,
    /// 客户ID
    pub customer_id: i32,
    /// 客户名称（关联查询返回）
    pub customer_name: Option<String>,
    /// 合同总金额
    pub total_amount: f64,
    /// 付款条款
    pub payment_terms: Option<String>,
    /// 交货日期
    pub delivery_date: String,
    /// 合同状态
    pub status: String,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 创建用户ID
    pub created_by: Option<i32>,
}

/// 销售合同查询参数
#[derive(Debug, Clone, Serialize)]
pub struct SalesContractQueryParams {
    /// 搜索关键词
    pub keyword: Option<String>,
    /// 合同状态
    pub status: Option<String>,
    /// 客户ID
    pub customer_id: Option<i32>,
    /// 页码
    pub page: Option<i64>,
    /// 每页数量
    pub page_size: Option<i64>,
}

impl Default for SalesContractQueryParams {
    fn default() -> Self {
        Self {
            keyword: None,
            status: None,
            customer_id: None,
            page: Some(1),
            page_size: Some(10),
        }
    }
}

/// 创建销售合同请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateSalesContractRequest {
    /// 合同编号
    pub contract_no: String,
    /// 合同名称
    pub contract_name: String,
    /// 客户ID
    pub customer_id: i32,
    /// 合同总金额
    pub total_amount: f64,
    /// 付款条款
    pub payment_terms: Option<String>,
    /// 交货日期
    pub delivery_date: String,
    /// 备注
    pub remark: Option<String>,
}

/// 合同执行请求
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteSalesContractRequest {
    /// 执行类型
    pub execution_type: String,
    /// 执行金额
    pub execution_amount: f64,
    /// 关联单据类型
    pub related_bill_type: Option<String>,
    /// 关联单据ID
    pub related_bill_id: Option<i32>,
    /// 备注
    pub remark: Option<String>,
}

/// 取消合同请求
#[derive(Debug, Clone, Serialize)]
pub struct CancelSalesContractRequest {
    /// 取消原因
    pub reason: String,
}

/// 销售合同列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct SalesContractListResponse {
    /// 合同列表
    pub data: Vec<SalesContract>,
    /// 总数
    pub total: i64,
}

/// 通用API响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

/// 销售合同服务
pub struct SalesContractService;

impl SalesContractService {
    /// 获取销售合同列表
    pub async fn list_contracts(params: SalesContractQueryParams) -> Result<SalesContractListResponse, String> {
        let mut url = String::from("/sales-contracts?");
        
        if let Some(keyword) = &params.keyword {
            url.push_str(&format!("keyword={}&", keyword));
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

        ApiService::get::<ApiResponse<SalesContractListResponse>>(&url)
            .await
            .map(|res| res.data)
    }

    /// 获取销售合同详情
    #[allow(dead_code)]
    pub async fn get_contract(id: i32) -> Result<SalesContract, String> {
        ApiService::get::<ApiResponse<SalesContract>>(&format!("/sales-contracts/{}", id))
            .await
            .map(|res| res.data)
    }

    /// 创建销售合同
    pub async fn create_contract(req: CreateSalesContractRequest) -> Result<SalesContract, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post::<ApiResponse<SalesContract>>("/sales-contracts", &payload)
            .await
            .map(|res| res.data)
    }

    /// 审核销售合同
    pub async fn approve_contract(id: i32) -> Result<String, String> {
        ApiService::put::<ApiResponse<String>>(&format!("/sales-contracts/{}/approve", id), &serde_json::json!({}))
            .await
            .map(|res| res.data)
    }

    /// 执行销售合同
    pub async fn execute_contract(id: i32, req: ExecuteSalesContractRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put::<ApiResponse<String>>(&format!("/sales-contracts/{}/execute", id), &payload)
            .await
            .map(|res| res.data)
    }

    /// 取消销售合同
    pub async fn cancel_contract(id: i32, reason: String) -> Result<String, String> {
        let req = CancelSalesContractRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put::<ApiResponse<String>>(&format!("/sales-contracts/{}/cancel", id), &payload)
            .await
            .map(|res| res.data)
    }
}