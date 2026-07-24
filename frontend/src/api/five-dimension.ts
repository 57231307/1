import { request } from './request'
import type { ApiResponse } from '@/types/api'

// 统计数据类型
export type StatsData = Record<string, string | number | boolean | null>

// 元数据类型
export type Metadata = Record<string, string | number | boolean | null>

// 仓库分布项
export interface WarehouseDistributionItem {
  warehouse_id: number
  warehouse_name: string
  quantity: number
  meters?: number
  kg?: number
}

export interface FiveDimensionStats {
  dimensionId?: number
  dimensionName?: string
  five_dimension_id?: number
  product_id?: number
  product_name?: string
  batch_no?: string
  color_no?: string
  dye_lot_no?: string
  grade?: string
  stats: StatsData
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
  metadata: Metadata
}

export interface FiveDimensionStatsResponse {
  dimension: FiveDimensionStats
  list: FiveDimensionItem[]
  total: number
  total_meters?: number
  total_kg?: number
  stock_count?: number
  warehouse_distribution?: WarehouseDistributionItem[]
}

export interface FiveDimensionItem {
  id: number
  name: string
  product_id?: number
  batch_no?: string
  color_no?: string
  dye_lot_no?: string
  grade?: string
}

// 统计查询参数
export interface StatsQueryParams {
  dimensionId?: number
  period?: string
  startDate?: string
  endDate?: string
  page?: number
  page_size?: number
  product_id?: number
  batch_no?: string
  color_no?: string
  grade?: string
}

// 搜索查询参数
export interface SearchQueryParams {
  q?: string
  dimensionId?: number
  type?: string
  page?: number
  page_size?: number
  keyword?: string
  search_type?: string
}

export const getFiveDimensionStatsList = (params?: StatsQueryParams) =>
  request.get('/crm/five-dimension/stats', { params })
export const getStatsByFiveDimensionId = (id: number) =>
  request.get(`/crm/five-dimension/stats/${id}`)
export const parseFiveDimensionId = (id: number | string) =>
  request.get(`/crm/five-dimension/parse/${id}`)
export const searchFiveDimension = (params?: SearchQueryParams) =>
  request.get('/crm/five-dimension/search', { params })

export const fiveDimensionApi = {
  getStats: (dimensionId: number, params?: FiveDimensionQuery) =>
    request.get<ApiResponse<FiveDimensionStats>>(`/crm/five-dimension/stats/${dimensionId}`, {
      params,
    }),

  listStats: (params?: FiveDimensionQuery) =>
    request.get<ApiResponse<{ stats: FiveDimensionStats[] }>>('/crm/five-dimension/stats', {
      params,
    }),

  search: (query: string, params?: FiveDimensionQuery) =>
    request.get<ApiResponse<{ results: FiveDimensionSearchResult[] }>>(
      '/crm/five-dimension/search',
      {
        params: { ...params, q: query },
      }
    ),

  getByDimensionId: (dimensionId: number) =>
    request.get<ApiResponse<FiveDimensionStats>>(`/crm/five-dimension/${dimensionId}`),
}
