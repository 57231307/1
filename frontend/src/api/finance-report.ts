import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 以下接口逐字段镜像 backend/src/models/dto/finance_report_dto.rs。
// 金额字段对应 Rust `Decimal`（未启 serde-floats，序列化为 JSON 十进制字符串）：
// 前端声明为 string，展示/求和/图表入参一律走 utils/money.ts 归一，禁止直接 .toFixed 或 +。
// Option<T> 字段在前端以可选（?）表达；NOT NULL 字段不得标 ?。
// 非金额整型（level/line_no/customer_id/supplier_id 等对应 i32）保持 number。

/** 报表项目（Rust: finance_report_dto.rs:56 ReportItem） */
export interface ReportItem {
  name: string;
  amount: string;
  description?: string;
}

/** 资产负债表（Rust: finance_report_dto.rs:11 BalanceSheet） */
export interface BalanceSheet {
  assets: ReportItem[];
  total_assets: string;
  liabilities: ReportItem[];
  total_liabilities: string;
  equity: ReportItem[];
  total_equity: string;
  report_date: string;
}

/** 利润表（Rust: finance_report_dto.rs:23 IncomeStatement） */
export interface IncomeStatement {
  revenue: ReportItem[];
  total_revenue: string;
  cost_of_goods_sold: string;
  gross_profit: string;
  operating_expenses: ReportItem[];
  total_operating_expenses: string;
  operating_income: string;
  other_income: string;
  other_expenses: string;
  net_income: string;
  period_start: string;
  period_end: string;
}

/** 现金流量表（Rust: finance_report_dto.rs:40 CashFlowStatement） */
export interface CashFlowStatement {
  operating_activities: ReportItem[];
  net_cash_from_operations: string;
  investing_activities: ReportItem[];
  net_cash_from_investing: string;
  financing_activities: ReportItem[];
  net_cash_from_financing: string;
  net_change_in_cash: string;
  beginning_cash: string;
  ending_cash: string;
  period_start: string;
  period_end: string;
}

/** 试算平衡表条目（Rust: finance_report_dto.rs:64 TrialBalanceEntry） */
export interface TrialBalanceEntry {
  subject_code: string;
  subject_name: string;
  /** Rust: i32 */
  level: number;
  initial_debit: string;
  initial_credit: string;
  period_debit: string;
  period_credit: string;
  ending_debit: string;
  ending_credit: string;
}

/** 试算平衡表（Rust: finance_report_dto.rs:78 TrialBalance） */
export interface TrialBalance {
  entries: TrialBalanceEntry[];
  total_initial_debit: string;
  total_initial_credit: string;
  total_period_debit: string;
  total_period_credit: string;
  total_ending_debit: string;
  total_ending_credit: string;
  period: string;
}

/** 总账条目（Rust: finance_report_dto.rs:91 GeneralLedgerEntry） */
export interface GeneralLedgerEntry {
  voucher_date: string;
  voucher_no: string;
  /** Rust: i32 */
  line_no: number;
  summary?: string;
  debit: string;
  credit: string;
  direction: string;
  balance: string;
}

/** 总账（Rust: finance_report_dto.rs:104 GeneralLedger） */
export interface GeneralLedger {
  subject_code: string;
  subject_name: string;
  entries: GeneralLedgerEntry[];
  opening_balance: string;
  closing_balance: string;
  total_debit: string;
  total_credit: string;
  period_start: string;
  period_end: string;
}

/** 明细账条目（Rust: finance_report_dto.rs:118 SubsidiaryLedgerEntry） */
export interface SubsidiaryLedgerEntry {
  business_date: string;
  business_no: string;
  business_type: string;
  subject_code: string;
  subject_name: string;
  summary?: string;
  debit: string;
  credit: string;
  /** Rust: Option<i32> */
  customer_id?: number;
  /** Rust: Option<i32> */
  supplier_id?: number;
}

/** 明细账（Rust: finance_report_dto.rs:133 SubsidiaryLedger） */
export interface SubsidiaryLedger {
  dimension_type: string;
  dimension_value: string;
  entries: SubsidiaryLedgerEntry[];
  total_debit: string;
  total_credit: string;
  period_start: string;
  period_end: string;
}

// 财务报表查询参数
export interface FinanceReportQueryParams {
  period?: string;
  start_date?: string;
  end_date?: string;
  company_id?: number;
  department_id?: number;
}

// 总账查询参数
export interface GeneralLedgerQueryParams extends FinanceReportQueryParams {
  subject_code?: string;
}

// 明细账查询参数
export interface SubsidiaryLedgerQueryParams extends FinanceReportQueryParams {
  customer_id?: number;
  supplier_id?: number;
  subject_code?: string;
}

// 后端 finance_report_handler::get_balance_sheet 无 Query<T> 提取器（资产负债表为全量快照，
// 不接受期间/日期区间），前端 params 会被 Axum 丢弃，故不再发送。
export function getBalanceSheet() {
  return request.get<ApiResponse<BalanceSheet>>('/finance/reports/balance-sheet');
}

export function getProfitStatement(params?: FinanceReportQueryParams) {
  return request.get<ApiResponse<IncomeStatement>>('/finance/reports/income-statement', { params });
}

export function getCashFlowStatement(params?: FinanceReportQueryParams) {
  return request.get<ApiResponse<CashFlowStatement>>('/finance/reports/cash-flow', { params });
}

export function getTrialBalance(params?: FinanceReportQueryParams) {
  return request.get<ApiResponse<TrialBalance>>('/finance/reports/trial-balance', { params });
}

export function getGeneralLedger(accountSubjectCode: string, params?: GeneralLedgerQueryParams) {
  return request.get<ApiResponse<GeneralLedger>>(
    `/finance/reports/general-ledger/${accountSubjectCode}`,
    { params }
  );
}

export function getSubsidiaryLedger(
  customerId?: number,
  supplierId?: number,
  params?: SubsidiaryLedgerQueryParams
) {
  return request.get<ApiResponse<SubsidiaryLedger>>('/finance/reports/subsidiary-ledger', {
    params: { customer_id: customerId, supplier_id: supplierId, ...params },
  });
}
