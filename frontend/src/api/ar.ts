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

export interface ARPayment {
  id: number
  payment_no: string
  customer_id: number
  customer_name: string
  payment_date: string
  payment_amount: number
  payment_method: string
  status: string
  bank_account?: string
  remark?: string
  created_at: string
}

export interface ARVerification {
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

export function listARPayments(params?: QueryParams): Promise<ApiResponse<ARPayment[]>> {
  return request.get('/ar/payments', { params })
}

export function getARPayment(id: number): Promise<ApiResponse<ARPayment>> {
  return request.get(`/ar/payments/${id}`)
}

export function createARPayment(data: Partial<ARPayment>): Promise<ApiResponse<ARPayment>> {
  return request.post('/ar/payments', data)
}

export function updateARPayment(
  id: number,
  data: Partial<ARPayment>
): Promise<ApiResponse<ARPayment>> {
  return request.put(`/ar/payments/${id}`, data)
}

export function confirmARPayment(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/payments/${id}/confirm`)
}

export function listARVerifications(params?: QueryParams): Promise<ApiResponse<ARVerification[]>> {
  return request.get('/ar/verifications', { params })
}

export function getARVerification(id: number): Promise<ApiResponse<ARVerification>> {
  return request.get(`/ar/verifications/${id}`)
}

export function autoVerifyAR(data: {
  invoice_id: number
  payment_id?: number
}): Promise<ApiResponse<ARVerification>> {
  return request.post('/ar/verifications/auto', data)
}

export function manualVerifyAR(data: {
  invoice_id: number
  payment_id: number
  amount: number
}): Promise<ApiResponse<ARVerification>> {
  return request.post('/ar/verifications/manual', data)
}

export function cancelARVerification(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/verifications/${id}/cancel`)
}

export function getUnverifiedARInvoices(): Promise<ApiResponse<ARInvoice[]>> {
  return request.get('/ar/verifications/unverified/invoices')
}

export function getUnverifiedARPayments(): Promise<ApiResponse<ARPayment[]>> {
  return request.get('/ar/verifications/unverified/payments')
}

export function getARStatisticsReport(params?: QueryParams): Promise<ApiResponse<any>> {
  return request.get('/ar/reports/statistics', { params })
}

export function getARDailyReport(date: string): Promise<ApiResponse<any>> {
  return request.get('/ar/reports/daily', { params: { date } })
}

export function getARMonthlyReport(year: number, month: number): Promise<ApiResponse<any>> {
  return request.get('/ar/reports/monthly', { params: { year, month } })
}

export function getARAgingReport(): Promise<ApiResponse<any>> {
  return request.get('/ar/reports/aging')
}
