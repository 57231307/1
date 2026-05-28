import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface ARInvoice {
  id: number
  invoice_no: string
  customer_id: number
  customer_name: string
  invoice_date: string
  invoice_amount: number
  tax_amount: number
  verified_amount: number
  unverified_amount: number
  status: string
  payment_status: string
  due_date?: string
  remark?: string
  created_at: string
}

export interface ARReconciliation {
  id: number
  reconciliation_no: string
  customer_id: number
  customer_name: string
  reconciliation_date: string
  total_invoice_amount: number
  total_payment_amount: number
  difference_amount: number
  status: string
  confirmed_by?: string
  confirmed_at?: string
  created_at: string
}

export interface FundAccount {
  id: number
  account_code: string
  account_name: string
  account_type: string
  balance: number
  frozen_balance: number
  available_balance: number
  status: string
  bank_name?: string
  bank_account?: string
  created_at: string
}

export function listARInvoices(params?: QueryParams): Promise<ApiResponse<ARInvoice[]>> {
  return request.get('/ar/invoices', { params })
}

export function getARInvoice(id: number): Promise<ApiResponse<ARInvoice>> {
  return request.get(`/ar/invoices/${id}`)
}

export function createARInvoice(data: Partial<ARInvoice>): Promise<ApiResponse<ARInvoice>> {
  return request.post('/ar/invoices', data)
}

export function updateARInvoice(
  id: number,
  data: Partial<ARInvoice>
): Promise<ApiResponse<ARInvoice>> {
  return request.put(`/ar/invoices/${id}`, data)
}

export function deleteARInvoice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/ar/invoices/${id}`)
}

export function approveARInvoice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/invoices/${id}/approve`)
}

export function cancelARInvoice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/invoices/${id}/cancel`)
}

export function listARReconciliations(
  params?: QueryParams
): Promise<ApiResponse<ARReconciliation[]>> {
  return request.get('/ar-reconciliations', { params })
}

export function getARReconciliation(id: number): Promise<ApiResponse<ARReconciliation>> {
  return request.get(`/ar-reconciliations/${id}`)
}

export function createARReconciliation(
  data: Partial<ARReconciliation>
): Promise<ApiResponse<ARReconciliation>> {
  return request.post('/ar-reconciliations', data)
}

export function updateARReconciliationStatus(
  id: number,
  status: string
): Promise<ApiResponse<void>> {
  return request.put(`/ar-reconciliations/${id}/status`, { status })
}

export function listFundAccounts(params?: QueryParams): Promise<ApiResponse<FundAccount[]>> {
  return request.get('/fund-management/accounts', { params })
}

export function getFundAccount(id: number): Promise<ApiResponse<FundAccount>> {
  return request.get(`/fund-management/accounts/${id}`)
}

export function createFundAccount(data: Partial<FundAccount>): Promise<ApiResponse<FundAccount>> {
  return request.post('/fund-management/accounts', data)
}

export function depositFund(
  id: number,
  data: { amount: number; remark?: string }
): Promise<ApiResponse<FundAccount>> {
  return request.post(`/fund-management/accounts/${id}/deposit`, data)
}

export function withdrawFund(
  id: number,
  data: { amount: number; remark?: string }
): Promise<ApiResponse<FundAccount>> {
  return request.post(`/fund-management/accounts/${id}/withdraw`, data)
}

export function freezeFund(
  id: number,
  data: { amount: number; reason: string }
): Promise<ApiResponse<FundAccount>> {
  return request.post(`/fund-management/accounts/${id}/freeze`, data)
}

export function unfreezeFund(
  id: number,
  data: { amount: number }
): Promise<ApiResponse<FundAccount>> {
  return request.post(`/fund-management/accounts/${id}/unfreeze`, data)
}

export function deleteFundAccount(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/fund-management/accounts/${id}`)
}

export function transferFund(data: {
  from_account_id: number
  to_account_id: number
  amount: number
  remark?: string
}): Promise<ApiResponse<void>> {
  return request.post('/fund-management/transfer', data)
}
