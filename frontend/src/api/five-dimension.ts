import { request } from './request'
import type { ApiResponse } from './request'

export interface FiveDimensionStats {
  dimensionId?: number
  dimensionName?: string
  stats: Record<string, any>
  period?: string
}

export interface FiveDimensionQuery {
  dimensionId?: number
  startDate?: string
  endDate?: string
  metrics?: string[]
}

export interface FiveDimensionSearchResult {
  id: number
  name: string
  type: string
  score: number
  metadata: Record<string, any>
}

export const fiveDimensionApi = {
  getStats: (dimensionId: number, params?: FiveDimensionQuery) =>
    request.get<ApiResponse<FiveDimensionStats>>(`/five-dimension/stats/${dimensionId}`, { params }),

  listStats: (params?: FiveDimensionQuery) =>
    request.get<ApiResponse<{ stats: FiveDimensionStats[] }>>('/five-dimension/stats', { params }),

  search: (query: string, params?: FiveDimensionQuery) =>
    request.get<ApiResponse<{ results: FiveDimensionSearchResult[] }>>('/five-dimension/search', { params: { ...params, q: query } }),

  getByDimensionId: (dimensionId: number) =>
    request.get<ApiResponse<FiveDimensionStats>>(`/five-dimension/${dimensionId}`),
}
