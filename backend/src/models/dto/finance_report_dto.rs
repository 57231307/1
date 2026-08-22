#![allow(dead_code)]
//! 财务报表 DTO
//!
//! 资产负债表、利润表、现金流量表等报表的数据结构

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 资产负债表
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub assets: Vec<ReportItem>,
    pub total_assets: Decimal,
    pub liabilities: Vec<ReportItem>,
    pub total_liabilities: Decimal,
    pub equity: Vec<ReportItem>,
    pub total_equity: Decimal,
    pub report_date: String,
}

/// 利润表
#[derive(Debug, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub revenue: Vec<ReportItem>,
    pub total_revenue: Decimal,
    pub cost_of_goods_sold: Decimal,
    pub gross_profit: Decimal,
    pub operating_expenses: Vec<ReportItem>,
    pub total_operating_expenses: Decimal,
    pub operating_income: Decimal,
    pub other_income: Decimal,
    pub other_expenses: Decimal,
    pub net_income: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 现金流量表
#[derive(Debug, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub operating_activities: Vec<ReportItem>,
    pub net_cash_from_operations: Decimal,
    pub investing_activities: Vec<ReportItem>,
    pub net_cash_from_investing: Decimal,
    pub financing_activities: Vec<ReportItem>,
    pub net_cash_from_financing: Decimal,
    pub net_change_in_cash: Decimal,
    pub beginning_cash: Decimal,
    pub ending_cash: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 报表项目
#[derive(Debug, Serialize, Deserialize)]
pub struct ReportItem {
    pub name: String,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// 试算平衡表条目
#[derive(Debug, Serialize, Deserialize)]
pub struct TrialBalanceEntry {
    pub subject_code: String,
    pub subject_name: String,
    pub level: i32,
    pub initial_debit: Decimal,
    pub initial_credit: Decimal,
    pub period_debit: Decimal,
    pub period_credit: Decimal,
    pub ending_debit: Decimal,
    pub ending_credit: Decimal,
}

/// 试算平衡表
#[derive(Debug, Serialize, Deserialize)]
pub struct TrialBalance {
    pub entries: Vec<TrialBalanceEntry>,
    pub total_initial_debit: Decimal,
    pub total_initial_credit: Decimal,
    pub total_period_debit: Decimal,
    pub total_period_credit: Decimal,
    pub total_ending_debit: Decimal,
    pub total_ending_credit: Decimal,
    pub period: String,
}

/// 总账条目
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralLedgerEntry {
    pub voucher_date: String,
    pub voucher_no: String,
    pub line_no: i32,
    pub summary: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub direction: String,
    pub balance: Decimal,
}

/// 总账
#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralLedger {
    pub subject_code: String,
    pub subject_name: String,
    pub entries: Vec<GeneralLedgerEntry>,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 明细账条目
#[derive(Debug, Serialize, Deserialize)]
pub struct SubsidiaryLedgerEntry {
    pub business_date: String,
    pub business_no: String,
    pub business_type: String,
    pub subject_code: String,
    pub subject_name: String,
    pub summary: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
}

/// 明细账
#[derive(Debug, Serialize, Deserialize)]
pub struct SubsidiaryLedger {
    pub dimension_type: String,
    pub dimension_value: String,
    pub entries: Vec<SubsidiaryLedgerEntry>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub period_start: String,
    pub period_end: String,
}

/// 凭证分录穿透明细（F-P2-2 修复，批次 387 v13 复审）（用于报表项目穿透到凭证分录级，包含业务单据追溯字段（source_type/source_bill_id），；前端可据此继续调用业务单据 API 完成全链路追溯。）
#[derive(Debug, Serialize, Deserialize)]
pub struct VoucherItemDetail {
    pub voucher_id: i32,
    pub voucher_no: String,
    pub voucher_date: chrono::NaiveDate,
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: String,
    pub summary: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
}

/// V15 P1 batch-19 缺陷 23.5.4：Incoterms 术语使用月报条目
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code, reason = "预留：Incoterms 术语统计条目，待接入")]
pub struct IncotermStatItem {
    /// Incoterms 代码（如 FOB / CIF / DDP）
    pub incoterm_code: String,
    /// 中文业务描述
    pub incoterm_description: String,
    /// 报价单数量
    pub quotation_count: i64,
    /// 报价总金额
    pub total_amount: Decimal,
    /// 运费成本合计
    pub total_freight_cost: Decimal,
    /// 保险费成本合计
    pub total_insurance_cost: Decimal,
    /// 关税成本合计
    pub total_duty_cost: Decimal,
    /// 金额占比（%）
    pub amount_percentage: Decimal,
}

/// V15 P1 batch-19 缺陷 23.5.4：Incoterms 术语使用月报
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code, reason = "预留：Incoterms 术语月报，待接入")]
pub struct IncotermMonthlyReport {
    /// 年份
    pub year: i32,
    /// 月份
    pub month: u32,
    /// 报价单总数
    pub total_quotations: i64,
    /// 报价总金额
    pub total_amount: Decimal,
    /// 按术语聚合的统计列表（按金额降序）
    pub items: Vec<IncotermStatItem>,
}
