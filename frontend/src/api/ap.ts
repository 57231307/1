import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

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
  created_at: string
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
  bank_account?: string
  remark?: string
  created_at: string
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
  created_at: string
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
  created_at: string
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
  created_at: string
}

export function listAPInvoices(params?: QueryParams): Promise<ApiResponse<APInvoice[]>> {
  return request.get('/ap/invoices', { params })
}

export function getAPInvoice(id: number): Promise<ApiResponse<APInvoice>> {
  return request.get(`/ap/invoices/${id}`)
}

export function createAPInvoice(data: Partial<APInvoice>): Promise<ApiResponse<APInvoice>> {
  return request.post('/ap/invoices', data)
}

export function updateAPInvoice(
  id: number,
  data: Partial<APInvoice>
): Promise<ApiResponse<APInvoice>> {
  return request.put(`/ap/invoices/${id}`, data)
}

export function deleteAPInvoice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/ap/invoices/${id}`)
}

export function approveAPInvoice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/invoices/${id}/approve`)
}

export function cancelAPInvoice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/invoices/${id}/cancel`)
}

export function autoGenerateAPInvoices(data: {
  order_ids: number[]
}): Promise<ApiResponse<{ invoice_ids: number[] }>> {
  return request.post('/ap/invoices/auto-generate', data)
}

export function getAPAgingAnalysis(params?: {
  supplier_id?: number
  date?: string
}): Promise<ApiResponse<any[]>> {
  return request.get('/ap/invoices/aging', { params })
}

export function listAPPayments(params?: QueryParams): Promise<ApiResponse<APPayment[]>> {
  return request.get('/ap/payments', { params })
}

export function getAPPayment(id: number): Promise<ApiResponse<APPayment>> {
  return request.get(`/ap/payments/${id}`)
}

export function createAPPayment(data: Partial<APPayment>): Promise<ApiResponse<APPayment>> {
  return request.post('/ap/payments', data)
}

export function updateAPPayment(
  id: number,
  data: Partial<APPayment>
): Promise<ApiResponse<APPayment>> {
  return request.put(`/ap/payments/${id}`, data)
}

export function confirmAPPayment(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/payments/${id}/confirm`)
}

export function listAPPaymentRequests(
  params?: QueryParams
): Promise<ApiResponse<APPaymentRequest[]>> {
  return request.get('/ap/payment-requests', { params })
}

export function getAPPaymentRequest(id: number): Promise<ApiResponse<APPaymentRequest>> {
  return request.get(`/ap/payment-requests/${id}`)
}

export function createAPPaymentRequest(
  data: Partial<APPaymentRequest>
): Promise<ApiResponse<APPaymentRequest>> {
  return request.post('/ap/payment-requests', data)
}

export function updateAPPaymentRequest(
  id: number,
  data: Partial<APPaymentRequest>
): Promise<ApiResponse<APPaymentRequest>> {
  return request.put(`/ap/payment-requests/${id}`, data)
}

export function deleteAPPaymentRequest(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/ap/payment-requests/${id}`)
}

export function submitAPPaymentRequest(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/payment-requests/${id}/submit`)
}

export function approveAPPaymentRequest(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/payment-requests/${id}/approve`)
}

export function rejectAPPaymentRequest(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/payment-requests/${id}/reject`, { reason })
}

export function listAPVerifications(params?: QueryParams): Promise<ApiResponse<APVerification[]>> {
  return request.get('/ap/verifications', { params })
}

export function getAPVerification(id: number): Promise<ApiResponse<APVerification>> {
  return request.get(`/ap/verifications/${id}`)
}

export function autoVerifyAP(data: {
  invoice_id: number
  payment_id?: number
}): Promise<ApiResponse<APVerification>> {
  return request.post('/ap/verifications/auto', data)
}

export function manualVerifyAP(data: {
  invoice_id: number
  payment_id: number
  amount: number
}): Promise<ApiResponse<APVerification>> {
  return request.post('/ap/verifications/manual', data)
}

export function cancelAPVerification(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/verifications/${id}/cancel`)
}

export function getUnverifiedAPInvoices(): Promise<ApiResponse<APInvoice[]>> {
  return request.get('/ap/verifications/unverified/invoices')
}

export function getUnverifiedAPPayments(): Promise<ApiResponse<APPayment[]>> {
  return request.get('/ap/verifications/unverified/payments')
}

export function listAPReconciliations(
  params?: QueryParams
): Promise<ApiResponse<APReconciliation[]>> {
  return request.get('/ap/reconciliations', { params })
}

export function getAPReconciliation(id: number): Promise<ApiResponse<APReconciliation>> {
  return request.get(`/ap/reconciliations/${id}`)
}

export function generateAPReconciliation(data: {
  supplier_id: number
  start_date: string
  end_date: string
}): Promise<ApiResponse<APReconciliation>> {
  return request.post('/ap/reconciliations/generate', data)
}

export function confirmAPReconciliation(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/reconciliations/${id}/confirm`)
}

export function disputeAPReconciliation(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/reconciliations/${id}/dispute`, { reason })
}

export function autoReconcileAllAP(): Promise<ApiResponse<void>> {
  return request.post('/ap/reconciliations/auto')
}

export function getAPSupplierSummary(supplierId: number): Promise<ApiResponse<any>> {
  return request.get(`/ap/reconciliations/summary`, { params: { supplier_id: supplierId } })
}

export function getAPInvoiceRelations(id: number): Promise<ApiResponse<any>> {
  return request.get(`/ap/invoices/${id}/relations`)
}

export function getAPStatisticsReport(params?: QueryParams): Promise<ApiResponse<any>> {
  return request.get('/ap/reports/statistics', { params })
}

export function getAPDailyReport(date: string): Promise<ApiResponse<any>> {
  return request.get('/ap/reports/daily', { params: { date } })
}

export function getAPMonthlyReport(year: number, month: number): Promise<ApiResponse<any>> {
  return request.get('/ap/reports/monthly', { params: { year, month } })
}

export function getAPAgingReport(): Promise<ApiResponse<any>> {
  return request.get('/ap/reports/aging')
}
