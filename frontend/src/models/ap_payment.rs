/// 付款数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
