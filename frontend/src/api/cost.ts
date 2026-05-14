import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

// 成本归集接口
export interface CostCollection {
  id: number
  collection_no: string
  cost_type: string
  period: string
  department_id: number
  department_name?: string
  total_cost: number
  status: 'draft' | 'pending' | 'approved' | 'rejected'
  remark?: string
  created_at?: string
  updated_at?: string
}

// 成本状态字典
export const COST_STATUS = {
  draft: { label: '草稿', type: 'info' },
  pending: { label: '待审核', type: 'warning' },
  approved: { label: '已批准', type: 'success' },
  rejected: { label: '已拒绝', type: 'danger' },
}

// 获取成本归集列表
export function listCostCollections(params?: QueryParams): Promise<ApiResponse<{ list: CostCollection[]; total: number }>> {
  return request.get('/cost-collections', { params })
}

// 获取成本归集详情
export function getCostCollection(id: number): Promise<ApiResponse<CostCollection>> {
  return request.get(`/cost-collections/${id}`)
}

// 创建成本归集
export function createCostCollection(data: Partial<CostCollection>): Promise<ApiResponse<CostCollection>> {
  return request.post('/cost-collections', data)
}

// 更新成本归集
export function updateCostCollection(id: number, data: Partial<CostCollection>): Promise<ApiResponse<CostCollection>> {
  return request.put(`/cost-collections/${id}`, data)
}

// 删除成本归集
export function deleteCostCollection(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/cost-collections/${id}`)
}

// 获取成本分析汇总
export function getCostAnalysisSummary(params?: { period?: string }): Promise<ApiResponse<any>> {
  return request.get('/cost-collections/analysis/summary', { params })
}

// 按批次获取成本
export function getCostByBatch(params?: { batch_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/cost-collections/analysis/by-batch', { params })
}
