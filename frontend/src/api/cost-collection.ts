import { request } from './request'
import type { ApiResponse } from './request'

export interface CostCollection {
  id?: number
  collectionName: string
  costType: string
  batchNo?: string
  productId?: number
  productName?: string
  totalCost?: number
  materialCost?: number
  laborCost?: number
  overheadCost?: number
  quantity?: number
  unitCost?: number
  status?: string
  collectionDate?: string
  createdBy?: number
  createdAt?: string
  updatedAt?: string
}

export interface CostAnalysisSummary {
  totalCost: number
  materialCost: number
  laborCost: number
  overheadCost: number
  unitCost: number
  costByType: Array<{ type: string; amount: number }>
  costByBatch: Array<{ batch: string; amount: number }>
}

export interface CostByBatch {
  batchNo: string
  productId: number
  productName: string
  quantity: number
  totalCost: number
  unitCost: number
  costBreakdown: {
    materialCost: number
    laborCost: number
    overheadCost: number
  }
}

export const costCollectionApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: CostCollection[]; total: number }>>('/cost-collections', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<CostCollection>>(`/cost-collections/${id}`),

  create: (data: Partial<CostCollection>) =>
    request.post<ApiResponse<CostCollection>>('/cost-collections', data),

  update: (id: number, data: Partial<CostCollection>) =>
    request.put<ApiResponse<CostCollection>>(`/cost-collections/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/cost-collections/${id}`),

  getCostAnalysisSummary: (params?: any) =>
    request.get<ApiResponse<CostAnalysisSummary>>('/cost-collections/analysis/summary', { params }),

  getCostByBatch: (params?: any) =>
    request.get<ApiResponse<{ batches: CostByBatch[] }>>('/cost-collections/analysis/by-batch', { params }),
}
