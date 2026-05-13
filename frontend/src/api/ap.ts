import { request } from './request'
import type { ApiResponse } from './request'

export interface APInvoice {
  id: number
  invoice_no: string
  supplier_id: number
  supplier_name: string
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

export interface APPayment {
  id: number
  payment_no: string
  supplier_id: number
  supplier_name: string
  payment_date: string
  payment_amount: number
  payment_method: string
  status: string
  remark?: string
}

export interface APPaymentRequest {
  id: number
  request_no: string
  supplier_id: number
  supplier_name: string
  request_amount: number
  approved_amount?: number
  request_date: string
  status: string
  payment_method?: string
  bank_account?: string
  remark?: string
}

export interface APVerification {
  id: number
  verification_no: string
  invoice_id: number
  invoice_no: string
  payment_id?: number
  payment_no?: string
  verification_amount: number
  verification_date: string
  status: string
}

export interface APReconciliation {
  id: number
  reconciliation_no: string
  supplier_id: number
  supplier_name: string
  reconciliation_date: string
  total_invoice_amount: number
  total_payment_amount: number
  difference_amount: number
  status: string
  confirmed_by?: string
  confirmed_at?: string
}

export const apApi = {
  listInvoices: (params?: any) =>
    request.get<ApiResponse<{ list: APInvoice[]; total: number }>>('/ap/invoices', { params }),

  createInvoice: (data: Partial<APInvoice>) =>
    request.post<ApiResponse<APInvoice>>('/ap/invoices', data),

  getInvoice: (id: number) =>
    request.get<ApiResponse<APInvoice>>(`/ap/invoices/${id}`),

  updateInvoice: (id: number, data: Partial<APInvoice>) =>
    request.put<ApiResponse<APInvoice>>(`/ap/invoices/${id}`, data),

  deleteInvoice: (id: number) =>
    request.delete<ApiResponse<null>>(`/ap/invoices/${id}`),

  approveInvoice: (id: number) =>
    request.post<ApiResponse<null>>(`/ap/invoices/${id}/approve`),

  cancelInvoice: (id: number) =>
    request.post<ApiResponse<null>>(`/ap/invoices/${id}/cancel`),

  autoGenerate: (data: { order_ids: number[] }) =>
    request.post<ApiResponse<{ invoice_ids: number[] }>>('/ap/invoices/auto-generate', data),

  getAgingAnalysis: (params?: { supplier_id?: number; date?: string }) =>
    request.get<ApiResponse<any[]>>('/ap/invoices/aging', { params }),

  listPayments: (params?: any) =>
    request.get<ApiResponse<{ list: APPayment[]; total: number }>>('/ap/payments', { params }),

  createPayment: (data: Partial<APPayment>) =>
    request.post<ApiResponse<APPayment>>('/ap/payments', data),

  getPayment: (id: number) =>
    request.get<ApiResponse<APPayment>>(`/ap/payments/${id}`),

  updatePayment: (id: number, data: Partial<APPayment>) =>
    request.put<ApiResponse<APPayment>>(`/ap/payments/${id}`, data),

  confirmPayment: (id: number) =>
    request.post<ApiResponse<null>>(`/ap/payments/${id}/confirm`),

  listPaymentRequests: (params?: any) =>
    request.get<ApiResponse<{ list: APPaymentRequest[]; total: number }>>('/ap/payment-requests', { params }),

  createPaymentRequest: (data: Partial<APPaymentRequest>) =>
    request.post<ApiResponse<APPaymentRequest>>('/ap/payment-requests', data),

  submitPaymentRequest: (id: number) =>
    request.post<ApiResponse<null>>(`/ap/payment-requests/${id}/submit`),

  approvePaymentRequest: (id: number) =>
    request.post<ApiResponse<null>>(`/ap/payment-requests/${id}/approve`),

  rejectPaymentRequest: (id: number, reason: string) =>
    request.post<ApiResponse<null>>(`/ap/payment-requests/${id}/reject`, { reason }),

  listVerifications: (params?: any) =>
    request.get<ApiResponse<{ list: APVerification[]; total: number }>>('/ap/verifications', { params }),

  getVerification: (id: number) =>
    request.get<ApiResponse<APVerification>>(`/ap/verifications/${id}`),

  autoVerify: (data: { invoice_id: number; payment_id?: number }) =>
    request.post<ApiResponse<APVerification>>('/ap/verifications/auto', data),

  manualVerify: (data: { invoice_id: number; payment_id: number; amount: number }) =>
    request.post<ApiResponse<APVerification>>('/ap/verifications/manual', data),

  listReconciliations: (params?: any) =>
    request.get<ApiResponse<{ list: APReconciliation[]; total: number }>>('/ap/reconciliations', { params }),

  getReconciliation: (id: number) =>
    request.get<ApiResponse<APReconciliation>>(`/ap/reconciliations/${id}`),

  confirmReconciliation: (id: number) =>
    request.post<ApiResponse<null>>(`/ap/reconciliations/${id}/confirm`),

  disputeReconciliation: (id: number, reason: string) =>
    request.post<ApiResponse<null>>(`/ap/reconciliations/${id}/dispute`, { reason }),

  getStatisticsReport: (params?: any) =>
    request.get<ApiResponse<any>>('/ap/reports/statistics', { params }),

  getDailyReport: (date: string) =>
    request.get<ApiResponse<any>>('/ap/reports/daily', { params: { date } }),

  getMonthlyReport: (year: number, month: number) =>
    request.get<ApiResponse<any>>('/ap/reports/monthly', { params: { year, month } }),

  getAgingReport: () =>
    request.get<ApiResponse<any>>('/ap/reports/aging'),
}
