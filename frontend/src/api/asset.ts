import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface FixedAsset {
  id: number
  asset_code: string
  asset_name: string
  category: string
  department_id?: number
  department_name?: string
  purchase_date: string
  purchase_amount: number
  salvage_value: number
  useful_life_months: number
  depreciation_method: string
  accumulated_depreciation: number
  net_value: number
  status: string
  location?: string
  custodian?: string
  created_at: string
  updated_at: string
}

export interface FixedAssetCreateRequest {
  asset_code: string
  asset_name: string
  category: string
  department_id?: number
  purchase_date: string
  purchase_amount: number
  salvage_value: number
  useful_life_months: number
  depreciation_method: string
  location?: string
  custodian?: string
}

export interface FixedAssetUpdateRequest {
  asset_name?: string
  department_id?: number
  location?: string
  custodian?: string
  status?: string
}

export function listAssets(params?: QueryParams): Promise<ApiResponse<FixedAsset[]>> {
  return request.get('/fixed-assets', { params })
}

export function getAsset(id: number): Promise<ApiResponse<FixedAsset>> {
  return request.get(`/fixed-assets/${id}`)
}

export function createAsset(data: FixedAssetCreateRequest): Promise<ApiResponse<FixedAsset>> {
  return request.post('/fixed-assets', data)
}

export function updateAsset(
  id: number,
  data: FixedAssetUpdateRequest
): Promise<ApiResponse<FixedAsset>> {
  return request.put(`/fixed-assets/${id}`, data)
}

export function deleteAsset(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/fixed-assets/${id}`)
}

export function depreciateAsset(id: number, period: string): Promise<ApiResponse<FixedAsset>> {
  // 批次 88 PH-2：补传 period 参数（YYYY-MM 格式），后端 DepreciateRequest 必填
  return request.post(`/fixed-assets/${id}/depreciate`, { period })
}

// 资产处置请求（对齐后端 DisposeRequest）
// v3 复审 P1-2：新增资产处置能力，支持出售/报废/转移
export interface DisposalRequest {
  disposal_type: string // SALE 出售 / SCRAP 报废 / TRANSFER 转移
  disposal_value: number
  disposal_date: string
  reason: string
  buyer_info?: string
}

// 资产处置：将指定资产标记为已处置并记录处置信息
export function disposeAsset(id: number, data: DisposalRequest): Promise<ApiResponse<string>> {
  return request.post(`/fixed-assets/${id}/dispose`, data)
}

export interface Budget {
  id: number
  budget_code: string
  budget_name: string
  budget_type: string
  department_id?: number
  department_name?: string
  fiscal_year: number
  total_amount: number
  used_amount: number
  remaining_amount: number
  status: string
  start_date: string
  end_date: string
  created_at: string
  updated_at: string
  items?: BudgetItem[]
}

export interface BudgetItem {
  id: number
  budget_id: number
  subject_id: number
  subject_code: string
  subject_name: string
  planned_amount: number
  used_amount: number
  remaining_amount: number
  month?: number
}

export interface BudgetCreateRequest {
  budget_code: string
  budget_name: string
  budget_type: string
  department_id?: number
  fiscal_year: number
  total_amount: number
  start_date: string
  end_date: string
  items?: {
    subject_id: number
    planned_amount: number
    month?: number
  }[]
}

export function listBudgets(params?: QueryParams): Promise<ApiResponse<Budget[]>> {
  return request.get('/budgets', { params })
}

export function getBudget(id: number): Promise<ApiResponse<Budget>> {
  return request.get(`/budgets/${id}`)
}

export function createBudget(data: BudgetCreateRequest): Promise<ApiResponse<Budget>> {
  return request.post('/budgets', data)
}

export function updateBudget(
  id: number,
  data: Partial<BudgetCreateRequest>
): Promise<ApiResponse<Budget>> {
  return request.put(`/budgets/${id}`, data)
}

export function deleteBudget(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/budgets/${id}`)
}

export function approveBudget(id: number): Promise<ApiResponse<void>> {
  return request.post(`/budgets/${id}/approve`)
}

export function adjustBudget(data: {
  budget_id: number
  adjustment_amount: number
  reason: string
}): Promise<ApiResponse<void>> {
  return request.post('/budgets/adjust', data)
}

export function listBudgetItems(params?: QueryParams): Promise<ApiResponse<BudgetItem[]>> {
  return request.get('/budgets/items', { params })
}

export const batchDepreciateAssets = (data: {
  asset_ids: number[]
  calculation_date: string
  user_id: number
}) => request.post('/fixed-assets/batch-depreciate', data)
