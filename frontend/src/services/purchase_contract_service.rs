//! 采购合同服务
//! 提供采购合同的增删改查等API调用

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 采购合同数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PurchaseContract {
    /// 合同ID
    pub id: i32,
    /// 合同编号
    pub contract_no: String,
    /// 合同名称
    pub contract_name: String,
    /// 供应商ID
    pub supplier_id: i32,
    /// 供应商名称（关联查询返回）
    pub supplier_name: Option<String>,
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

/// 采购合同查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseContractQueryParams {
    /// 搜索关键词
    pub keyword: Option<String>,
    /// 合同状态
    pub status: Option<String>,
    /// 供应商ID
    pub supplier_id: Option<i32>,
    /// 页码
    pub page: Option<i64>,
    /// 每页数量
    pub page_size: Option<i64>,
}

impl Default for PurchaseContractQueryParams {
    fn default() -> Self {
        Self {
            keyword: None,
            status: None,
            supplier_id: None,
            page: Some(1),
            page_size: Some(10),
        }
    }
}

/// 创建采购合同请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseContractRequest {
    /// 合同编号
    pub contract_no: String,
    /// 合同名称
    pub contract_name: String,
    /// 供应商ID
    pub supplier_id: i32,
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
pub struct ExecutePurchaseContractRequest {
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
pub struct CancelPurchaseContractRequest {
    /// 取消原因
    pub reason: String,
}

/// 采购合同列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct PurchaseContractListResponse {
    /// 合同列表
    pub data: Vec<PurchaseContract>,
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

/// 采购合同服务
pub struct PurchaseContractService;

impl PurchaseContractService {
    /// 获取采购合同列表
    pub async fn list_contracts(params: PurchaseContractQueryParams) -> Result<PurchaseContractListResponse, String> {
        let mut url = String::from("/purchase-contracts?");

        if let Some(keyword) = &params.keyword {
            url.push_str(&format!("keyword={}&", keyword));
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

        ApiService::get::<ApiResponse<PurchaseContractListResponse>>(&url)
            .await
            .map(|res| res.data)
    }

    /// 获取采购合同详情
    #[allow(dead_code)]
    pub async fn get_contract(id: i32) -> Result<PurchaseContract, String> {
        ApiService::get::<ApiResponse<PurchaseContract>>(&format!("/purchase-contracts/{}", id))
            .await
            .map(|res| res.data)
    }

    /// 创建采购合同
    pub async fn create_contract(req: CreatePurchaseContractRequest) -> Result<PurchaseContract, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post::<ApiResponse<PurchaseContract>>("/purchase-contracts", &payload)
            .await
            .map(|res| res.data)
    }

    /// 审核采购合同
    pub async fn approve_contract(id: i32) -> Result<String, String> {
        ApiService::put::<ApiResponse<String>>(&format!("/purchase-contracts/{}/approve", id), &serde_json::json!({}))
            .await
            .map(|res| res.data)
    }

    /// 执行采购合同
    pub async fn execute_contract(id: i32, req: ExecutePurchaseContractRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put::<ApiResponse<String>>(&format!("/purchase-contracts/{}/execute", id), &payload)
            .await
            .map(|res| res.data)
    }

    /// 取消采购合同
    pub async fn cancel_contract(id: i32, reason: String) -> Result<String, String> {
        let req = CancelPurchaseContractRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put::<ApiResponse<String>>(&format!("/purchase-contracts/{}/cancel", id), &payload)
            .await
            .map(|res| res.data)
    }
}
