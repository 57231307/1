import { request } from './request'
import type { ApiResponse } from './request'

export interface SupplierEvaluation {
  id?: number
  supplierId?: number
  supplierName?: string
  evaluationDate?: string
  score?: number
  level?: string
  status?: string
  items?: EvaluationItem[]
  remarks?: string
  evaluatorId?: number
  evaluatorName?: string
  createdAt?: string
  updatedAt?: string
}

export interface EvaluationItem {
  indicatorId?: number
  indicatorName?: string
  weight?: number
  score?: number
  weightedScore?: number
  comment?: string
}

export interface EvaluationIndicator {
  id?: number
  indicatorName: string
  indicatorCode: string
  category?: string
  weight?: number
  description?: string
  isActive?: boolean
}

export interface SupplierRanking {
  supplierId: number
  supplierName: string
  totalScore: number
  rank: number
  level: string
}

export interface SupplierEvaluationQueryParams {
  page?: number
  pageSize?: number
  supplierId?: number
  status?: string
  startDate?: string
  endDate?: string
}

export const supplierEvaluationApi = {
  list: (params?: SupplierEvaluationQueryParams) =>
    request.get<ApiResponse<{ list: SupplierEvaluation[]; total: number }>>('/supplier-evaluation/evaluations', { params }),

  create: (data: Partial<SupplierEvaluation>) =>
    request.post<ApiResponse<SupplierEvaluation>>('/supplier-evaluation/evaluations', data),

  getById: (id: number) =>
    request.get<ApiResponse<SupplierEvaluation>>(`/supplier-evaluation/evaluations/${id}`),

  update: (id: number, data: Partial<SupplierEvaluation>) =>
    request.put<ApiResponse<SupplierEvaluation>>(`/supplier-evaluation/evaluations/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/supplier-evaluation/evaluations/${id}`),

  listIndicators: () =>
    request.get<ApiResponse<{ indicators: EvaluationIndicator[] }>>('/supplier-evaluation/indicators'),

  getRankings: (params?: any) =>
    request.get<ApiResponse<{ rankings: SupplierRanking[] }>>('/supplier-evaluation/rankings', { params }),

  listRecords: (params?: any) =>
    request.get<ApiResponse<{ records: any[]; total: number }>>('/supplier-evaluation/records', { params }),
}
