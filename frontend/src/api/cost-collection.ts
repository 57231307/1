import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface CostCollection {
  id: number
  cost_type: string
  amount: number
  source_id: number
  source_type: string
  allocation_rule?: string
  created_at?: string
  updated_at?: string
}

export interface CostAllocationRequest {
  source_id: number
  target_ids: number[]
  allocation_method: string
}

export interface CostSummary {
  total_cost: number
  direct_cost: number
  indirect_cost: number
  allocated_cost: number
  unallocated_cost: number
}

export interface CostCollectionQueryParams extends QueryParams {
  cost_type?: string
  source_id?: number
  source_type?: string
}

export function listCostCollections(params?: CostCollectionQueryParams): Promise<ApiResponse<{ list: CostCollection[]; total: number }>> {
  return request.get('/cost-collections', { params })
}

export function getCostCollection(id: number): Promise<ApiResponse<CostCollection>> {
  return request.get(`/cost-collections/${id}`)
}

export function createCostCollection(data: Partial<CostCollection>): Promise<ApiResponse<CostCollection>> {
  return request.post('/cost-collections', data)
}

export function updateCostCollection(id: number, data: Partial<CostCollection>): Promise<ApiResponse<CostCollection>> {
  return request.put(`/cost-collections/${id}`, data)
}

export function deleteCostCollection(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/cost-collections/${id}`)
}

export function calculateCost(id: number): Promise<ApiResponse<{ calculated_amount: number }>> {
  return request.post(`/cost-collections/${id}/calculate`)
}

export function allocateCost(data: CostAllocationRequest): Promise<ApiResponse<{ allocated_ids: number[] }>> {
  return request.post('/cost-collections/allocate', data)
}

export function getCostSummary(params?: { period_start?: string; period_end?: string }): Promise<ApiResponse<CostSummary>> {
  return request.get('/cost-collections/summary', { params })
}
