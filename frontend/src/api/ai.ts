import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export function forecastSales(params?: { period: string; product_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/forecast-sales', { params })
}

export function optimizeInventory(params?: { warehouse_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/optimize-inventory', { params })
}

export function detectAnomalies(params?: { period: string; data_type: string }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/detect-anomalies', { params })
}

export function getRecommendations(params?: { user_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/recommendations', { params })
}
