import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface FiveDimension {
  id?: number
  fiveDimensionId: string
  productId?: number
  productName?: string
  productCode?: string
  batchNo?: string
  colorCode?: string
  colorName?: string
  width?: number
  weight?: number
  length?: number
  totalQuantity?: number
  availableQuantity?: number
  unit?: string
  warehouseId?: number
  warehouseName?: string
  status?: string
  createdAt?: string
  updatedAt?: string
}

export interface FiveDimensionStat {
  fiveDimensionId: string
  productName: string
  colorName: string
  specification: string
  totalQuantity: number
  availableQuantity: number
  warehouseCount: number
}

export interface FiveDimensionQueryParams extends QueryParams {
  fiveDimensionId?: string
  productId?: number
  batchNo?: string
  colorCode?: string
  warehouseId?: number
  status?: string
}

export interface ParseFiveDimensionRequest {
  fiveDimensionId: string
}

export function getStats(): Promise<ApiResponse<FiveDimensionStat>> {
  return request.get('/five-dimension/stats')
}

export function listFiveDimensions(params?: FiveDimensionQueryParams): Promise<ApiResponse<{ list: FiveDimension[]; total: number }>> {
  return request.get('/five-dimension/list', { params })
}

export function searchFiveDimensions(params?: { keyword?: string; productId?: number; colorCode?: string }): Promise<ApiResponse<FiveDimension[]>> {
  return request.get('/five-dimension/search', { params })
}

export function getByFiveDimensionId(fiveDimensionId: string): Promise<ApiResponse<FiveDimension>> {
  return request.get(`/five-dimension/${encodeURIComponent(fiveDimensionId)}`)
}

export function parseFiveDimension(data: ParseFiveDimensionRequest): Promise<ApiResponse<FiveDimension>> {
  return request.post('/five-dimension/parse', data)
}
