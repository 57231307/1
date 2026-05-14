import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface Budget {
  id: number
  budget_no: string
  name: string
  period: string
  department_id: number
  department_name?: string
  total_amount: number
  status: 'draft' | 'pending' | 'approved' | 'rejected'
  remark?: string
  created_at?: string
  updated_at?: string
}

export const BUDGET_STATUS = {
  draft: { label: '草稿', type: 'info' },
  pending: { label: '待审核', type: 'warning' },
  approved: { label: '已批准', type: 'success' },
  rejected: { label: '已拒绝', type: 'danger' },
}

export function listBudgets(params?: QueryParams): Promise<ApiResponse<{ list: Budget[]; total: number }>> {
  return request.get('/api/v1/erp/budgets', { params })
}

export function getBudget(id: number): Promise<ApiResponse<Budget>> {
  return request.get(`/api/v1/erp/budgets/${id}`)
}

export function createBudget(data: Partial<Budget>): Promise<ApiResponse<Budget>> {
  return request.post('/api/v1/erp/budgets', data)
}

export function updateBudget(id: number, data: Partial<Budget>): Promise<ApiResponse<Budget>> {
  return request.put(`/api/v1/erp/budgets/${id}`, data)
}

export function deleteBudget(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api/v1/erp/budgets/${id}`)
}

export function approveBudget(id: number): Promise<ApiResponse<void>> {
  return request.post(`/api/v1/erp/budgets/${id}/approve`)
}
