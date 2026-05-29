import { request } from './request'

export interface BalanceSheetItem {
  code: string
  name: string
  parent_code?: string
  level: number
  debit_amount: number
  credit_amount: number
  balance: number
}

export interface ProfitStatementItem {
  code: string
  name: string
  parent_code?: string
  level: number
  amount: number
}

export interface CashFlowItem {
  code: string
  name: string
  parent_code?: string
  level: number
  inflow: number
  outflow: number
  net_flow: number
}

export interface ReportData {
  period: string
  period_name: string
  items: any[]
  total?: number
}

export function getBalanceSheet(params?: any) {
  return request.get('/finance/reports/balance-sheet', { params })
}

export function getProfitStatement(params?: any) {
  return request.get('/finance/reports/income-statement', { params })
}

export function getCashFlowStatement(params?: any) {
  return request.get('/finance/reports/cash-flow', { params })
}

export function getTrialBalance(params?: any) {
  return request.get('/finance/reports/trial-balance', { params })
}

export function getGeneralLedger(accountSubjectCode: string, params?: any) {
  return request.get(`/finance/reports/general-ledger/${accountSubjectCode}`, { params })
}

export function getSubsidiaryLedger(customerId?: number, supplierId?: number, params?: any) {
  return request.get('/finance/reports/subsidiary-ledger', {
    params: { customer_id: customerId, supplier_id: supplierId, ...params },
  })
}
