#![allow(dead_code)]
//! 资金管理服务相关的数据传输对象（DTO）
//!
//! 包含资金账户查询/创建/更新请求、现金流预测、银企对账、资金日月报等纯数据结构。

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::models::fund_management;

/// 资金账户查询参数
#[derive(Debug, Clone, Default)]
pub struct FundAccountQueryParams {
    pub account_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建资金账户请求
#[derive(Debug, Clone)]
pub struct CreateFundAccountRequest {
    pub account_name: String,
    pub account_no: String,
    pub account_type: String,
    pub bank_name: Option<String>,
    pub currency: String,
    pub opened_date: Option<chrono::NaiveDate>,
    pub remark: Option<String>,
}

/// 更新资金账户请求
#[derive(Debug, Clone)]
pub struct UpdateFundAccountRequest {
    pub account_name: Option<String>,
    pub bank_name: Option<String>,
    pub currency: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// V15 P1 17.6-D2：现金流预测数据点
#[derive(Debug, Clone, serde::Serialize)]
pub struct CashFlowForecastPoint {
    /// 日期
    pub date: NaiveDate,
    /// 当日流入（应收到期）
    pub inflow: Decimal,
    /// 当日流出（应付到期）
    pub outflow: Decimal,
    /// 当日净流 = 流入 - 流出
    pub net_flow: Decimal,
    /// 累计预计余额（含期初余额）
    pub projected_balance: Decimal,
}

/// V15 P1 17.6-D3：账户 + 类型风控提示
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountWithTypeHint {
    /// 账户模型
    pub account: fund_management::Model,
    /// 是否需要银企对账
    pub reconciliation_required: bool,
    /// 风控提示
    pub control_hint: String,
}

/// V15 P1 17.6-D4：银企对账结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct BankReconciliationResult {
    /// 账户 ID
    pub account_id: i32,
    /// 账户编号
    pub account_no: String,
    /// 账户名称
    pub account_name: String,
    /// 对账单日期
    pub statement_date: NaiveDate,
    /// 系统余额
    pub system_balance: Decimal,
    /// 银行对账单余额
    pub bank_statement_balance: Decimal,
    /// 原始差异 = 银行余额 - 系统余额
    pub difference: Decimal,
    /// 在途差异 = 在途流入 - 在途流出
    pub timing_difference: Decimal,
    /// 调整后差异 = 原始差异 - 在途差异
    pub adjusted_difference: Decimal,
    /// 差异分类：balanced / system_missing / system_excess
    pub diff_type: String,
    /// 在途转出笔数
    pub pending_out_count: i64,
    /// 在途转入笔数
    pub pending_in_count: i64,
}

/// V15 P1 17.6-D6：资金日报 — 每个账户日维度摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountDailySummary {
    pub account_id: i32,
    pub account_no: String,
    pub account_name: String,
    pub account_type: String,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub net_change: Decimal,
    pub inflow_count: i64,
    pub outflow_count: i64,
}

/// V15 P1 17.6-D6：资金日报汇总
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyReportSummary {
    pub report_date: NaiveDate,
    pub accounts: Vec<AccountDailySummary>,
    pub total_opening_balance: Decimal,
    pub total_closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub total_net_change: Decimal,
}

/// V15 P1 17.6-D6：资金月报 — 每个月度维度摘要
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountMonthlySummary {
    pub account_id: i32,
    pub account_no: String,
    pub account_name: String,
    pub account_type: String,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub net_change: Decimal,
    pub daily_avg_balance: Decimal,
    pub transfer_count: i64,
    pub total_transfer_amount: Decimal,
}

/// V15 P1 17.6-D6：资金月报汇总
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonthlyReportSummary {
    pub year: i32,
    pub month: u32,
    pub accounts: Vec<AccountMonthlySummary>,
    pub total_opening_balance: Decimal,
    pub total_closing_balance: Decimal,
    pub total_inflow: Decimal,
    pub total_outflow: Decimal,
    pub total_net_change: Decimal,
    pub total_transfer_count: i64,
    pub total_transfer_amount: Decimal,
}
