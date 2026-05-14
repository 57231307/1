import { request } from './request'

export interface FiveDimensionItem {
  product_id: number
  product_name?: string
  batch_no: string
  color_no: string
  dye_lot_no?: string
  grade: string
  five_dimension_id: string
}

export interface WarehouseDistribution {
  warehouse_id: number
  warehouse_name: string
  quantity_meters: number
  quantity_kg: number
}

export interface FiveDimensionStatsResponse {
  dimension: FiveDimensionItem
  total_meters: number
  total_kg: number
  stock_count: number
  warehouse_distribution: WarehouseDistribution[]
}

export interface FiveDimensionListResponse {
  items: FiveDimensionStatsResponse[]
  total: number
  page: number
  page_size: number
}

export interface FiveDimensionStatsParams {
  product_id?: number
  batch_no?: string
  color_no?: string
  dye_lot_no?: string
  grade?: string
  warehouse_id?: number
  page?: number
  page_size?: number
}

export interface FiveDimensionSearchParams {
  keyword: string
  search_type: string
  page?: number
  page_size?: number
}

export interface FiveDimensionSearchResponse {
  items: FiveDimensionItem[]
  total: number
  page: number
  page_size: number
}

export interface FiveDimensionIdParseResponse {
  success: boolean
  dimension?: FiveDimensionItem
  error?: string
}

export function getFiveDimensionStats(params?: FiveDimensionStatsParams) {
  return request.get('/five-dimension/stats', { params })
}

export function getStatsByFiveDimensionId(fiveDimensionId: string) {
  return request.get(`/five-dimension/${fiveDimensionId}`)
}

export function parseFiveDimensionId(fiveDimensionId: string) {
  return request.post('/five-dimension/parse', { five_dimension_id: fiveDimensionId })
}

export function searchFiveDimension(params: FiveDimensionSearchParams) {
  return request.get('/five-dimension/search', { params })
}

export function listFiveDimensionStats(params?: FiveDimensionStatsParams) {
  return request.get('/five-dimension/list', { params })
}