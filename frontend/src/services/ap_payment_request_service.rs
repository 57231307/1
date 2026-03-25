//! 付款申请服务
//!
//! 与后端付款申请API交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 付款申请数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApPaymentRequest {
    /// 主键 ID
    pub id: i32,
    /// 付款申请单号
    pub request_no: String,
    /// 申请日期
    pub request_date: String,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 供应商名称
    pub supplier_name: Option<String>,
    /// 付款类型
    pub payment_type: String,
    /// 付款方式
    pub payment_method: String,
    /// 申请金额
    pub request_amount: String,
    /// 审批状态
    pub approval_status: String,
    /// 币种
    pub currency: String,
    /// 汇率
    pub exchange_rate: String,
    /// 外币金额
    pub request_amount_foreign: Option<String>,
    /// 期望付款日期
    pub expected_payment_date: Option<String>,
    /// 收款银行
    pub bank_name: Option<String>,
    /// 收款账号
    pub bank_account: Option<String>,
    /// 收款账户名
    pub bank_account_name: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
    /// 创建人 ID
    pub created_by: i32,
    /// 创建人名称
    pub creator_name: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新人 ID
    pub updated_by: Option<i32>,
    /// 更新时间
    pub updated_at: String,
    /// 提交人 ID
    pub submitted_by: Option<i32>,
    /// 提交人名称
    pub submitter_name: Option<String>,
    /// 提交时间
    pub submitted_at: Option<String>,
    /// 审批人 ID
    pub approved_by: Option<i32>,
    /// 审批人名称
    pub approver_name: Option<String>,
    /// 审批时间
    pub approved_at: Option<String>,
    /// 拒绝人 ID
    pub rejected_by: Option<i32>,
    /// 拒绝人名称
    pub rejecter_name: Option<String>,
    /// 拒绝时间
    pub rejected_at: Option<String>,
    /// 拒绝原因
    pub rejected_reason: Option<String>,
}

/// 付款申请明细项
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApPaymentRequestItem {
    /// 主键 ID
    pub id: i32,
    /// 付款申请 ID
    pub request_id: i32,
    /// 应付单 ID
    pub invoice_id: i32,
    /// 应付单号
    pub invoice_no: Option<String>,
    /// 申请金额
    pub apply_amount: String,
    /// 备注
    pub notes: Option<String>,
}

/// 付款申请列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApPaymentRequestListResponse {
    /// 数据列表
    pub items: Vec<ApPaymentRequest>,
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
}

/// 付款申请查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ApPaymentRequestQueryParams {
    /// 供应商 ID
    pub supplier_id: Option<i32>,
    /// 审批状态
    pub approval_status: Option<String>,
    /// 付款类型
    pub payment_type: Option<String>,
    /// 开始日期
    pub start_date: Option<String>,
    /// 结束日期
    pub end_date: Option<String>,
    /// 页码
    pub page: Option<u64>,
    /// 每页大小
    pub page_size: Option<u64>,
}

/// 创建付款申请请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateApPaymentRequest {
    /// 供应商 ID
    pub supplier_id: i32,
    /// 申请日期
    pub request_date: String,
    /// 付款类型
    pub payment_type: String,
    /// 付款方式
    pub payment_method: String,
    /// 申请金额
    pub request_amount: String,
    /// 币种
    pub currency: Option<String>,
    /// 汇率
    pub exchange_rate: Option<String>,
    /// 期望付款日期
    pub expected_payment_date: Option<String>,
    /// 收款银行
    pub bank_name: Option<String>,
    /// 收款账号
    pub bank_account: Option<String>,
    /// 收款账户名
    pub bank_account_name: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
    /// 付款申请明细
    pub items: Vec<ApPaymentRequestItemRequest>,
}

/// 更新付款申请请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateApPaymentRequest {
    /// 申请日期
    pub request_date: Option<String>,
    /// 付款类型
    pub payment_type: Option<String>,
    /// 付款方式
    pub payment_method: Option<String>,
    /// 申请金额
    pub request_amount: Option<String>,
    /// 期望付款日期
    pub expected_payment_date: Option<String>,
    /// 收款银行
    pub bank_name: Option<String>,
    /// 收款账号
    pub bank_account: Option<String>,
    /// 收款账户名
    pub bank_account_name: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
}

/// 付款申请明细项请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ApPaymentRequestItemRequest {
    /// 应付单 ID
    pub invoice_id: i32,
    /// 申请金额
    pub apply_amount: String,
    /// 备注
    pub notes: Option<String>,
}

/// 拒绝付款申请请求
#[derive(Debug, Clone, Serialize)]
pub struct RejectApPaymentRequest {
    /// 拒绝原因
    pub reason: String,
}

/// 付款申请服务
pub struct ApPaymentRequestService;

impl ApPaymentRequestService {
    /// 查询付款申请列表
    pub async fn list_requests(params: ApPaymentRequestQueryParams) -> Result<ApPaymentRequestListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref status) = params.approval_status {
            query_parts.push(format!("approval_status={}", status));
        }
        if let Some(ref ptype) = params.payment_type {
            query_parts.push(format!("payment_type={}", ptype));
        }
        if let Some(ref sd) = params.start_date {
            query_parts.push(format!("start_date={}", sd));
        }
        if let Some(ref ed) = params.end_date {
            query_parts.push(format!("end_date={}", ed));
        }
        if let Some(p) = params.page {
            query_parts.push(format!("page={}", p));
        }
        if let Some(ps) = params.page_size {
            query_parts.push(format!("page_size={}", ps));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/ap-payment-requests{}", query_string);
        ApiService::get::<ApPaymentRequestListResponse>(&url).await
    }

    /// 获取付款申请详情
    #[allow(dead_code)]
    pub async fn get_request(id: i32) -> Result<ApPaymentRequest, String> {
        ApiService::get::<ApPaymentRequest>(&format!("/ap-payment-requests/{}", id)).await
    }

    /// 创建付款申请
    #[allow(dead_code)]
    pub async fn create_request(req: CreateApPaymentRequest) -> Result<ApPaymentRequest, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-payment-requests", &payload).await
    }

    /// 更新付款申请
    #[allow(dead_code)]
    pub async fn update_request(id: i32, req: UpdateApPaymentRequest) -> Result<ApPaymentRequest, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap-payment-requests/{}", id), &payload).await
    }

    /// 删除付款申请
    pub async fn delete_request(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap-payment-requests/{}", id)).await
    }

    /// 提交付款申请
    pub async fn submit_request(id: i32) -> Result<ApPaymentRequest, String> {
        ApiService::post::<ApPaymentRequest>(&format!("/ap-payment-requests/{}/submit", id), &serde_json::json!({})).await
    }

    /// 审批付款申请
    pub async fn approve_request(id: i32) -> Result<ApPaymentRequest, String> {
        ApiService::post::<ApPaymentRequest>(&format!("/ap-payment-requests/{}/approve", id), &serde_json::json!({})).await
    }

    /// 拒绝付款申请
    pub async fn reject_request(id: i32, reason: String) -> Result<ApPaymentRequest, String> {
        let req = RejectApPaymentRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap-payment-requests/{}/reject", id), &payload).await
    }
}