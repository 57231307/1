//! 付款服务
//!
//! 与后端付款API交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 付款数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApPayment {
    /// 主键 ID
    pub id: i32,
    /// 付款单号
    pub payment_no: String,
    /// 付款申请 ID
    pub request_id: i32,
    /// 付款申请单号
    pub request_no: Option<String>,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 供应商名称
    pub supplier_name: Option<String>,
    /// 付款日期
    pub payment_date: String,
    /// 付款类型
    pub payment_type: String,
    /// 付款方式
    pub payment_method: String,
    /// 付款金额
    pub payment_amount: String,
    /// 付款状态
    pub payment_status: String,
    /// 币种
    pub currency: String,
    /// 汇率
    pub exchange_rate: String,
    /// 外币金额
    pub payment_amount_foreign: Option<String>,
    /// 收款银行
    pub bank_name: Option<String>,
    /// 收款账号
    pub bank_account: Option<String>,
    /// 收款账户名
    pub bank_account_name: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 付款确认人 ID
    pub confirmed_by: Option<i32>,
    /// 付款确认人名称
    pub confirmer_name: Option<String>,
    /// 付款确认时间
    pub confirmed_at: Option<String>,
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
}

/// 付款明细项
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApPaymentItem {
    /// 主键 ID
    pub id: i32,
    /// 付款 ID
    pub payment_id: i32,
    /// 应付单 ID
    pub invoice_id: i32,
    /// 应付单号
    pub invoice_no: Option<String>,
    /// 付款金额
    pub payment_amount: String,
    /// 备注
    pub notes: Option<String>,
}

/// 付款列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApPaymentListResponse {
    /// 数据列表
    pub items: Vec<ApPayment>,
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
}

/// 付款查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ApPaymentQueryParams {
    /// 供应商 ID
    pub supplier_id: Option<i32>,
    /// 付款状态
    pub payment_status: Option<String>,
    /// 付款方式
    pub payment_method: Option<String>,
    /// 开始日期
    pub start_date: Option<String>,
    /// 结束日期
    pub end_date: Option<String>,
    /// 页码
    pub page: Option<u64>,
    /// 每页大小
    pub page_size: Option<u64>,
}

/// 创建付款请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateApPaymentRequest {
    /// 付款申请 ID
    pub request_id: i32,
    /// 付款日期
    pub payment_date: String,
    /// 付款方式
    pub payment_method: String,
    /// 付款金额
    pub payment_amount: String,
    /// 币种
    pub currency: Option<String>,
    /// 汇率
    pub exchange_rate: Option<String>,
    /// 收款银行
    pub bank_name: Option<String>,
    /// 收款账号
    pub bank_account: Option<String>,
    /// 收款账户名
    pub bank_account_name: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 更新付款请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateApPaymentRequest {
    /// 付款日期
    pub payment_date: Option<String>,
    /// 付款方式
    pub payment_method: Option<String>,
    /// 付款金额
    pub payment_amount: Option<String>,
    /// 收款银行
    pub bank_name: Option<String>,
    /// 收款账号
    pub bank_account: Option<String>,
    /// 收款账户名
    pub bank_account_name: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 付款计划项
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaymentScheduleItem {
    /// 供应商 ID
    pub supplier_id: i32,
    /// 供应商名称
    pub supplier_name: String,
    /// 计划付款日期
    pub planned_date: String,
    /// 计划付款金额
    pub planned_amount: String,
    /// 付款申请单号
    pub request_no: String,
}

/// 付款服务
pub struct ApPaymentService;

impl ApPaymentService {
    /// 查询付款列表
    pub async fn list_payments(params: ApPaymentQueryParams) -> Result<ApPaymentListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref status) = params.payment_status {
            query_parts.push(format!("payment_status={}", status));
        }
        if let Some(ref method) = params.payment_method {
            query_parts.push(format!("payment_method={}", method));
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

        let url = format!("/ap-payments{}", query_string);
        ApiService::get::<ApPaymentListResponse>(&url).await
    }

    /// 获取付款详情
    #[allow(dead_code)]
    pub async fn get_payment(id: i32) -> Result<ApPayment, String> {
        ApiService::get::<ApPayment>(&format!("/ap-payments/{}", id)).await
    }

    /// 创建付款
    #[allow(dead_code)]
    pub async fn create_payment(req: CreateApPaymentRequest) -> Result<ApPayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-payments", &payload).await
    }

    /// 更新付款
    #[allow(dead_code)]
    pub async fn update_payment(id: i32, req: UpdateApPaymentRequest) -> Result<ApPayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap-payments/{}", id), &payload).await
    }

    /// 删除付款
    pub async fn delete_payment(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap-payments/{}", id)).await
    }

    /// 确认付款
    pub async fn confirm_payment(id: i32) -> Result<ApPayment, String> {
        ApiService::post::<ApPayment>(&format!("/ap-payments/{}/confirm", id), &serde_json::json!({})).await
    }

    /// 获取付款计划
    #[allow(dead_code)]
    pub async fn get_payment_schedule(
        supplier_id: Option<i32>,
        start_date: String,
        end_date: String,
    ) -> Result<Vec<PaymentScheduleItem>, String> {
        let mut query_parts = vec![];
        query_parts.push(format!("start_date={}", start_date));
        query_parts.push(format!("end_date={}", end_date));

        if let Some(sid) = supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }

        let query_string = format!("?{}", query_parts.join("&"));
        let url = format!("/ap-payments/schedule{}", query_string);
        ApiService::get::<Vec<PaymentScheduleItem>>(&url).await
    }
}