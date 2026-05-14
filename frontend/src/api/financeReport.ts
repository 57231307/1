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

export interface QueryParams {
  year?: number
  month?: number
  period_id?: number
}

export function getBalanceSheet(params?: QueryParams) {
  return request.get('/api/v1/finance-report/balance-sheet', { params })
}

export function getProfitStatement(params?: QueryParams) {
  return request.get('/api/v1/finance-report/profit-statement', { params })
}

export function getCashFlowStatement(params?: QueryParams) {
  return request.get('/api/v1/finance-report/cash-flow', { params })
}

export function getTrialBalance(params?: QueryParams) {
  return request.get('/api/v1/finance-report/trial-balance', { params })
}

export function getGeneralLedger(accountSubjectCode: string, params?: QueryParams) {
  return request.get(`/api/v1/finance-report/general-ledger/${accountSubjectCode}`, { params })
}

export function getSubsidiaryLedger(customerId?: number, supplierId?: number, params?: QueryParams) {
  return request.get('/api/v1/finance-report/subsidiary-ledger', { 
    params: { customer_id: customerId, supplier_id: supplierId, ...params } 
  })
}