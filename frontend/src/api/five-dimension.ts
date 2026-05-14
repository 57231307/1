import { request } from './request'
import type { ApiResponse } from './request'

export interface FiveDimensionStats {
  five_dimension_id: string
  product_name: string
  batch_no: string
  warehouse_name: string
  customer_name: string
  total_qty: number
  total_amount: number
}

export interface FiveDimensionSearchParams {
  product_id?: number
  batch_no?: string
  warehouse_id?: number
  customer_id?: number
  date_range?: string[]
}

export interface FiveDimensionTrace {
  id: number
  five_dimension_id: string
  operation_type: string
  operation_time: string
  operator: string
  description: string
}

export interface FiveDimensionParseResult {
  five_dimension_id: string
  product_id: number
  product_name: string
  batch_no: string
  color_no: string
  dye_lot_no?: string
  grade: string
}

export interface FiveDimensionListResponse {
  list: FiveDimensionStats[]
  total: number
  page: number
  page_size: number
}

export function getFiveDimensionStats(): Promise<ApiResponse<FiveDimensionStats[]>> {
  return request.get('/five-dimension/stats')
}

export function getStatsByFiveDimensionId(fiveDimensionId: string): Promise<ApiResponse<FiveDimensionStats>> {
  return request.get(`/five-dimension/${encodeURIComponent(fiveDimensionId)}`)
}

export function parseFiveDimensionId(fiveDimensionId: string): Promise<ApiResponse<FiveDimensionParseResult>> {
  return request.post('/five-dimension/parse', { five_dimension_id: fiveDimensionId })
}

export function searchFiveDimension(params?: FiveDimensionSearchParams): Promise<ApiResponse<FiveDimensionStats[]>> {
  return request.get('/five-dimension/search', { params })
}

export function listFiveDimensionStats(params?: FiveDimensionSearchParams): Promise<ApiResponse<FiveDimensionListResponse>> {
  return request.get('/five-dimension/list', { params })
}

export function getFiveDimensionTrace(fiveDimensionId: string): Promise<ApiResponse<FiveDimensionTrace[]>> {
  return request.get(`/five-dimension/${encodeURIComponent(fiveDimensionId)}/trace`)
}