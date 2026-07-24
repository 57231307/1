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

// P2-9c 修复（批次 82 v1 复审）：应收对账列表查询参数强类型化
export interface ArReconciliationQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  customer_id?: number
  customer_name?: string
  status?: string
  start_date?: string
  end_date?: string
}

export function getArReconciliationList(params?: ArReconciliationQueryParams) {
  return request.get('/ar-reconciliations', { params })
}

export function getArReconciliation(id: number) {
  return request.get(`/ar-reconciliations/${id}`)
}

export function createArReconciliation(data: Partial<ArReconciliationEntity>) {
  return request.post('/ar-reconciliations', data)
}

export function updateArReconciliation(id: number, data: Partial<ArReconciliationEntity>) {
  return request.put(`/ar-reconciliations/${id}`, data)
}

export function deleteArReconciliation(id: number) {
  return request.delete(`/ar-reconciliations/${id}`)
}

export function confirmReconciliation(id: number) {
  return request.put(`/ar-reconciliations/${id}/status`, { status: 'confirmed' })
}

export function getReconciliationDetails(id: number) {
  return request.get(`/ar-reconciliations/${id}`)
}

export function createReconciliationDetail(id: number, data: Partial<ReconciliationDetail>) {
  return request.post(`/ar-reconciliations/${id}`, data)
}

export function deleteReconciliationDetail(detailId: number) {
  return request.delete(`/ar-reconciliations/${detailId}`)
}
