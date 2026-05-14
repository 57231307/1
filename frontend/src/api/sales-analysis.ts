import { request } from './request'
import type { ApiResponse } from './request'

export interface SalesContract {
  id: number
  contract_no: string
  customer_id: number
  customer_name: string
  contract_date: string
  total_amount: number
  signed_amount?: number
  status: string
  start_date?: string
  end_date?: string
  payment_terms?: string
  delivery_terms?: string
  remark?: string
  creator_name?: string
  created_at?: string
}

export interface SalesAnalysis {
  id: number
  analysis_type: string
  period: string
  amount: number
  count: number
  compared_amount?: number
  growth_rate?: number
}

export interface SalesTarget {
  id: number
  target_year: number
  target_month?: number
  department_id?: number
  department_name?: string
  user_id?: number
  user_name?: string
  target_amount: number
  achieved_amount?: number
  achievement_rate?: number
  status: string
}

export const salesContractApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: SalesContract[]; total: number }>>('/sales-contracts', { params }),

  create: (data: Partial<SalesContract>) =>
    request.post<ApiResponse<SalesContract>>('/sales-contracts', data),

  get: (id: number) =>
    request.get<ApiResponse<SalesContract>>(`/sales-contracts/${id}`),

  update: (id: number, data: Partial<SalesContract>) =>
    request.put<ApiResponse<SalesContract>>(`/sales-contracts/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<null>>(`/sales-contracts/${id}`),

  approve: (id: number) =>
    request.post<ApiResponse<null>>(`/sales-contracts/${id}/approve`),

  execute: (id: number) =>
    request.put<ApiResponse<null>>(`/sales-contracts/${id}/execute`),

  cancel: (id: number) =>
    request.put<ApiResponse<null>>(`/sales-contracts/${id}/cancel`),
}

export const salesAnalysisApi = {
  getStatistics: (params?: { period?: string; group_by?: string }) =>
    request.get<ApiResponse<SalesAnalysis[]>>('/sales-analysis/statistics', { params }),

  getTrends: (params?: { period?: string; compare_period?: string }) =>
    request.get<ApiResponse<SalesAnalysis[]>>('/sales-analysis/trends', { params }),

  getRankings: (params?: { period?: string; top?: number }) =>
    request.get<ApiResponse<SalesAnalysis[]>>('/sales-analysis/rankings', { params }),

  getTargets: (params?: { year?: number }) =>
    request.get<ApiResponse<SalesTarget[]>>('/sales-analysis/targets', { params }),

  createTarget: (data: Partial<SalesTarget>) =>
    request.post<ApiResponse<SalesTarget>>('/sales-analysis/targets', data),
}
