import { request } from './request'
import type { ApiResponse } from './request'

export interface FixedAsset {
  id: number
  asset_code: string
  asset_name: string
  asset_type: string
  purchase_date: string
  purchase_amount: number
  useful_life: number
  residual_value: number
  depreciation_method: string
  accumulated_depreciation: number
  net_value: number
  location?: string
  custodian?: string
  status: string
  remark?: string
}

export interface Budget {
  id: number
  budget_code: string
  budget_name: string
  budget_year: number
  budget_period: string
  department_id?: number
  department_name?: string
  total_amount: number
  used_amount: number
  available_amount: number
  status: string
}

export interface BudgetItem {
  id: number
  budget_id: number
  item_code: string
  item_name: string
  amount: number
  used_amount: number
}

export const assetApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: FixedAsset[]; total: number }>>('/fixed-assets', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<FixedAsset>>(`/fixed-assets/${id}`),

  create: (data: Partial<FixedAsset>) =>
    request.post<ApiResponse<FixedAsset>>('/fixed-assets', data),

  update: (id: number, data: Partial<FixedAsset>) =>
    request.put<ApiResponse<FixedAsset>>(`/fixed-assets/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<null>>(`/fixed-assets/${id}`),

  depreciate: (id: number, period: string) =>
    request.post<ApiResponse<any>>(`/fixed-assets/${id}/depreciate`, { period }),
}

export const budgetApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: Budget[]; total: number }>>('/budgets', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<Budget>>(`/budgets/${id}`),

  create: (data: Partial<Budget>) =>
    request.post<ApiResponse<Budget>>('/budgets', data),

  update: (id: number, data: Partial<Budget>) =>
    request.put<ApiResponse<Budget>>(`/budgets/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<null>>(`/budgets/${id}`),

  approve: (id: number) =>
    request.post<ApiResponse<null>>(`/budgets/${id}/approve`),

  adjust: (id: number, data: { adjustment_amount: number; reason: string }) =>
    request.post<ApiResponse<null>>('/budgets/adjust', { budget_id: id, ...data }),

  listBudgetItems: (params?: any) =>
    request.get<ApiResponse<{ list: BudgetItem[]; total: number }>>('/budgets/items', { params }),

  createBudgetItem: (data: Partial<BudgetItem>) =>
    request.post<ApiResponse<BudgetItem>>('/budgets/items', data),

  listPlans: (params?: any) =>
    request.get<ApiResponse<{ list: any[]; total: number }>>('/budgets/plans', { params }),

  createPlan: (data: any) =>
    request.post<ApiResponse<any>>('/budgets/plans', data),

  approvePlan: (id: number) =>
    request.post<ApiResponse<null>>(`/budgets/plans/${id}/approve`),

  executePlan: (id: number) =>
    request.post<ApiResponse<null>>(`/budgets/plans/${id}/execute`),

  getControl: (planId: number) =>
    request.get<ApiResponse<any>>(`/budgets/control/${planId}`),

  getBudgetControlData: (planId: number) =>
    request.get<ApiResponse<any>>(`/budgets/control/${planId}/data`),
}
