import { request } from './request'

export interface ArReconciliationEntity {
  id?: number
  customer_id: number
  customer_name?: string
  customer_code?: string
  start_date: string
  end_date: string
  status: string
  total_invoice: number
  total_payment: number
  total_adjustment: number
  balance: number
  created_at?: string
  created_by?: number
  created_by_name?: string
  confirmed_at?: string
}

export interface ReconciliationDetail {
  id?: number
  reconciliation_id: number
  type: string
  source_no: string
  source_date: string
  amount: number
  paid_amount: number
  balance: number
  remark?: string
}

export interface QueryParams {
  page?: number
  pageSize?: number
  customer_id?: number
  customer_name?: string
  status?: string
  start_date?: string
  end_date?: string
}

export function listArReconciliations(params?: QueryParams) {
  return request.get('/ar-reconciliation', { params })
}

export function getArReconciliation(id: number) {
  return request.get(`/ar-reconciliation/${id}`)
}

export function createArReconciliation(data: Partial<ArReconciliationEntity>) {
  return request.post('/ar-reconciliation', data)
}

export function updateArReconciliation(id: number, data: Partial<ArReconciliationEntity>) {
  return request.put(`/ar-reconciliation/${id}`, data)
}

export function deleteArReconciliation(id: number) {
  return request.delete(`/ar-reconciliation/${id}`)
}

export function confirmReconciliation(id: number) {
  return request.patch(`/ar-reconciliation/${id}/confirm`)
}

export function getReconciliationDetails(id: number) {
  return request.get(`/ar-reconciliation/${id}/details`)
}

export function addReconciliationDetail(id: number, data: Partial<ReconciliationDetail>) {
  return request.post(`/ar-reconciliation/${id}/details`, data)
}

export function deleteReconciliationDetail(detailId: number) {
  return request.delete(`/ar-reconciliation/details/${detailId}`)
}