import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface BalanceSheetItem {
  code: string;
  name: string;
  parent_code?: string;
  level: number;
  debit_amount: number;
  credit_amount: number;
  balance: number;
}

export interface ProfitStatementItem {
  code: string;
  name: string;
  parent_code?: string;
  level: number;
  amount: number;
}

export interface CashFlowItem {
  code: string;
  name: string;
  parent_code?: string;
  level: number;
  inflow: number;
  outflow: number;
  net_flow: number;
}

export interface TrialBalanceItem {
  code: string;
  name: string;
  debit_amount: number;
  credit_amount: number;
  balance: number;
}

export interface GeneralLedgerItem {
  date: string;
  voucher_no: string;
  summary: string;
  debit_amount: number;
  credit_amount: number;
  balance: number;
  direction: string;
}

export interface SubsidiaryLedgerItem {
  date: string;
  voucher_no: string;
  summary: string;
  debit_amount: number;
  credit_amount: number;
  balance: number;
  direction: string;
  counterpart: string;
}

// 报表项联合类型
export type ReportItem =
  | BalanceSheetItem
  | ProfitStatementItem
  | CashFlowItem
  | TrialBalanceItem
  | GeneralLedgerItem
  | SubsidiaryLedgerItem;

export interface ReportData {
  period: string;
  period_name: string;
  items: ReportItem[];
  total?: number;
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
  return request.get<ApiResponse<ReportData>>('/finance/reports/balance-sheet');
}

export function getProfitStatement(params?: FinanceReportQueryParams) {
  return request.get<ApiResponse<ReportData>>('/finance/reports/income-statement', { params });
}

export function getCashFlowStatement(params?: FinanceReportQueryParams) {
  return request.get<ApiResponse<ReportData>>('/finance/reports/cash-flow', { params });
}

export function getTrialBalance(params?: FinanceReportQueryParams) {
  return request.get<ApiResponse<ReportData>>('/finance/reports/trial-balance', { params });
}

export function getGeneralLedger(accountSubjectCode: string, params?: GeneralLedgerQueryParams) {
  return request.get<ApiResponse<ReportData>>(
    `/finance/reports/general-ledger/${accountSubjectCode}`,
    { params }
  );
}

export function getSubsidiaryLedger(
  customerId?: number,
  supplierId?: number,
  params?: SubsidiaryLedgerQueryParams
) {
  return request.get<ApiResponse<ReportData>>('/finance/reports/subsidiary-ledger', {
    params: { customer_id: customerId, supplier_id: supplierId, ...params },
  });
}
