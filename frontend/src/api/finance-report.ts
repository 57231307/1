import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 以下接口逐字段镜像 backend/src/models/dto/finance_report_dto.rs。
// 金额字段对应 Rust `Decimal`（本仓前端约定统一声明为 number，渲染处按 number 归一处理）。
// Option<T> 字段在前端以可选（?）表达；NOT NULL 字段不得标 ?。

/** 报表项目（Rust: finance_report_dto.rs:56 ReportItem） */
export interface ReportItem {
  name: string;
  amount: number;
  description?: string;
}

/** 资产负债表（Rust: finance_report_dto.rs:11 BalanceSheet） */
export interface BalanceSheet {
  assets: ReportItem[];
  total_assets: number;
  liabilities: ReportItem[];
  total_liabilities: number;
  equity: ReportItem[];
  total_equity: number;
  report_date: string;
}

/** 利润表（Rust: finance_report_dto.rs:23 IncomeStatement） */
export interface IncomeStatement {
  revenue: ReportItem[];
  total_revenue: number;
  cost_of_goods_sold: number;
  gross_profit: number;
  operating_expenses: ReportItem[];
  total_operating_expenses: number;
  operating_income: number;
  other_income: number;
  other_expenses: number;
  net_income: number;
  period_start: string;
  period_end: string;
}

/** 现金流量表（Rust: finance_report_dto.rs:40 CashFlowStatement） */
export interface CashFlowStatement {
  operating_activities: ReportItem[];
  net_cash_from_operations: number;
  investing_activities: ReportItem[];
  net_cash_from_investing: number;
  financing_activities: ReportItem[];
  net_cash_from_financing: number;
  net_change_in_cash: number;
  beginning_cash: number;
  ending_cash: number;
  period_start: string;
  period_end: string;
}

/** 试算平衡表条目（Rust: finance_report_dto.rs:64 TrialBalanceEntry） */
export interface TrialBalanceEntry {
  subject_code: string;
  subject_name: string;
  level: number;
  initial_debit: number;
  initial_credit: number;
  period_debit: number;
  period_credit: number;
  ending_debit: number;
  ending_credit: number;
}

/** 试算平衡表（Rust: finance_report_dto.rs:78 TrialBalance） */
export interface TrialBalance {
  entries: TrialBalanceEntry[];
  total_initial_debit: number;
  total_initial_credit: number;
  total_period_debit: number;
  total_period_credit: number;
  total_ending_debit: number;
  total_ending_credit: number;
  period: string;
}

/** 总账条目（Rust: finance_report_dto.rs:91 GeneralLedgerEntry） */
export interface GeneralLedgerEntry {
  voucher_date: string;
  voucher_no: string;
  line_no: number;
  summary?: string;
  debit: number;
  credit: number;
  direction: string;
  balance: number;
}

/** 总账（Rust: finance_report_dto.rs:104 GeneralLedger） */
export interface GeneralLedger {
  subject_code: string;
  subject_name: string;
  entries: GeneralLedgerEntry[];
  opening_balance: number;
  closing_balance: number;
  total_debit: number;
  total_credit: number;
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
  debit: number;
  credit: number;
  customer_id?: number;
  supplier_id?: number;
}

/** 明细账（Rust: finance_report_dto.rs:133 SubsidiaryLedger） */
export interface SubsidiaryLedger {
  dimension_type: string;
  dimension_value: string;
  entries: SubsidiaryLedgerEntry[];
  total_debit: number;
  total_credit: number;
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
