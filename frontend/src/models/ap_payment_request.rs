/// 付款申请数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApPaymentRequestItemRequest {
    /// 应付单 ID
    pub invoice_id: i32,
    /// 申请金额
    pub apply_amount: String,
    /// 备注
    pub notes: Option<String>,
}

/// 拒绝付款申请请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct RejectApPaymentRequest {
    /// 拒绝原因
    pub reason: String,
}
