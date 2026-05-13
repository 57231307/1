import { request } from './request'
import type { ApiResponse } from './request'

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
}

export const arApi = {
  listInvoices: (params?: any) =>
    request.get<ApiResponse<{ list: ARInvoice[]; total: number }>>('/ar/invoices', { params }),

  createInvoice: (data: Partial<ARInvoice>) =>
    request.post<ApiResponse<ARInvoice>>('/ar/invoices', data),

  getInvoice: (id: number) =>
    request.get<ApiResponse<ARInvoice>>(`/ar/invoices/${id}`),

  updateInvoice: (id: number, data: Partial<ARInvoice>) =>
    request.put<ApiResponse<ARInvoice>>(`/ar/invoices/${id}`, data),

  deleteInvoice: (id: number) =>
    request.delete<ApiResponse<null>>(`/ar/invoices/${id}`),

  approveInvoice: (id: number) =>
    request.post<ApiResponse<null>>(`/ar/invoices/${id}/approve`),

  cancelInvoice: (id: number) =>
    request.post<ApiResponse<null>>(`/ar/invoices/${id}/cancel`),

  listReconciliations: (params?: any) =>
    request.get<ApiResponse<{ list: ARReconciliation[]; total: number }>>('/ar-reconciliations', { params }),

  createReconciliation: (data: Partial<ARReconciliation>) =>
    request.post<ApiResponse<ARReconciliation>>('/ar-reconciliations', data),

  getReconciliation: (id: number) =>
    request.get<ApiResponse<ARReconciliation>>(`/ar-reconciliations/${id}`),

  updateReconciliationStatus: (id: number, status: string) =>
    request.put<ApiResponse<null>>(`/ar-reconciliations/${id}/status`, { status }),
}

export const fundApi = {
  listAccounts: (params?: any) =>
    request.get<ApiResponse<{ list: FundAccount[]; total: number }>>('/fund-management/accounts', { params }),

  createAccount: (data: Partial<FundAccount>) =>
    request.post<ApiResponse<FundAccount>>('/fund-management/accounts', data),

  getAccount: (id: number) =>
    request.get<ApiResponse<FundAccount>>(`/fund-management/accounts/${id}`),

  deposit: (id: number, data: { amount: number; remark?: string }) =>
    request.post<ApiResponse<any>>(`/fund-management/accounts/${id}/deposit`, data),

  withdraw: (id: number, data: { amount: number; remark?: string }) =>
    request.post<ApiResponse<any>>(`/fund-management/accounts/${id}/withdraw`, data),

  freeze: (id: number, data: { amount: number; reason: string }) =>
    request.post<ApiResponse<any>>(`/fund-management/accounts/${id}/freeze`, data),

  unfreeze: (id: number, data: { amount: number }) =>
    request.post<ApiResponse<any>>(`/fund-management/accounts/${id}/unfreeze`, data),

  deleteAccount: (id: number) =>
    request.delete<ApiResponse<null>>(`/fund-management/accounts/${id}`),

  transfer: (data: { from_account_id: number; to_account_id: number; amount: number; remark?: string }) =>
    request.post<ApiResponse<any>>('/fund-management/transfer', data),
}
