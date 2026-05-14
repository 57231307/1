import { request } from './request'
import type { ApiResponse } from '@/types/api'

export function forecastSales(params?: { period: string; product_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/ai/forecast-sales', { params })
}

export function optimizeInventory(params?: { warehouse_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/ai/optimize-inventory', { params })
}

export function detectAnomalies(params?: { period: string; data_type: string }): Promise<ApiResponse<any>> {
  return request.get('/ai/detect-anomalies', { params })
}

export function getRecommendations(params?: { user_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/ai/recommendations', { params })
}
