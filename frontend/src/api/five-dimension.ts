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

export const listFiveDimensionStats = (params?: any) => request.get("/five-dimension/stats", { params })
export const getStatsByFiveDimensionId = (id: number) => request.get(`/five-dimension/stats/${id}`)
export const parseFiveDimensionId = (id: number | string) => request.get(`/five-dimension/parse/${id}`)
export const searchFiveDimension = (params?: any) => request.get("/five-dimension/search", { params })
export interface FiveDimensionStatsResponse { 
  dimension: any; 
  list: any[]; 
  total: number;
  total_meters?: number;
  total_kg?: number;
  stock_count?: number;
  warehouse_distribution?: any[];
}
export interface FiveDimensionItem { id: number; name: string; product_id?: number; batch_no?: string; color_no?: string; dye_lot_no?: string; grade?: string }

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
