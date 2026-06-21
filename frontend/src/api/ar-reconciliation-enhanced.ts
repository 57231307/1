import { request } from './request'
import type { ApiResponse, PageResult } from '@/types/api'

export interface AgingBucket {
  label: string
  range: string
  amount: number
  percentage: number
  count: number
}

export interface AgingAnalysisResult {
  customer_id: number
  customer_name: string
  customer_code: string
  total_amount: number
  buckets: AgingBucket[]
  analyzed_at: string
}

export interface AutoReconciliationResult {
  id: number
  customer_id: number
  customer_name: string
  customer_code: string
  match_status: 'matched' | 'partial' | 'unmatched'
  invoice_amount: number
  payment_amount: number
  difference: number
  matched_count: number
  unmatched_count: number
  created_at: string
}

export interface ReconciliationDetailItem {
  id: number
  reconciliation_id: number
  type: 'invoice' | 'payment' | 'adjustment'
  source_no: string
  source_date: string
  amount: number
  matched_amount: number
  unmatched_amount: number
  status: 'matched' | 'partial' | 'unmatched'
  remark?: string
}

export interface CustomerConfirmation {
  id: number
  reconciliation_id: number
  customer_id: number
  customer_name: string
  confirm_status: 'pending' | 'confirmed' | 'disputed'
  confirm_amount: number
  disputed_amount: number
  confirmed_at?: string
  confirmed_by?: string
  remark?: string
}

export interface DisputeRecord {
  id: number
  confirmation_id: number
  reconciliation_id: number
  dispute_type: 'amount' | 'quality' | 'delivery' | 'other'
  dispute_amount: number
  description: string
  status: 'open' | 'investigating' | 'resolved' | 'closed'
  resolution?: string
  created_at: string
  resolved_at?: string
  created_by?: string
}

export function autoReconcile(params: {
  start_date: string
  end_date: string
  customer_id?: number
}): Promise<ApiResponse<{ task_id: number; message: string }>> {
  return request.post('/ar-reconciliations-enhanced/auto-match', params)
}

export function getAutoReconciliationResults(
  params?: any
): Promise<ApiResponse<PageResult<AutoReconciliationResult>>> {
  return request.get('/ar-reconciliations-enhanced/auto-match', { params })
}

export function getAgingAnalysis(params?: {
  customer_id?: number
  as_of_date?: string
}): Promise<ApiResponse<AgingAnalysisResult[]>> {
  return request.get('/ar-reconciliations-enhanced/aging-report', { params })
}

export function getReconciliationDetailItems(
  id: number
): Promise<ApiResponse<ReconciliationDetailItem[]>> {
  return request.get(`/ar-reconciliations-enhanced/${id}/details`)
}

export function sendCustomerConfirmation(id: number): Promise<ApiResponse<{ message: string }>> {
  return request.post(`/ar-reconciliations-enhanced/${id}/confirm/send`)
}

export function getCustomerConfirmations(
  params?: any
): Promise<ApiResponse<PageResult<CustomerConfirmation>>> {
  return request.get('/ar-reconciliations-enhanced/confirmations', { params })
}

export function updateConfirmationStatus(
  id: number,
  data: { status: 'confirmed' | 'disputed'; remark?: string }
): Promise<ApiResponse<CustomerConfirmation>> {
  return request.put(`/ar-reconciliations-enhanced/confirmations/${id}/status`, data)
}

export function createDispute(data: Partial<DisputeRecord>): Promise<ApiResponse<DisputeRecord>> {
  return request.post('/ar-reconciliations-enhanced/disputes', data)
}

export function getDisputes(params?: any): Promise<ApiResponse<PageResult<DisputeRecord>>> {
  return request.get('/ar-reconciliations-enhanced/disputes', { params })
}

export function resolveDispute(
  id: number,
  data: { resolution: string }
): Promise<ApiResponse<DisputeRecord>> {
  return request.put(`/ar-reconciliations-enhanced/disputes/${id}/resolve`, data)
}

export function getDisputeDetail(id: number): Promise<ApiResponse<DisputeRecord>> {
  return request.get(`/ar-reconciliations-enhanced/disputes/${id}`)
}
