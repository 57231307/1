import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface SalesPrice {
  id: number
  product_id: number
  product_name: string
  product_code: string
  customer_id: number
  customer_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  expiry_date: string
  status: 'active' | 'inactive'
  remark: string
  created_at: string
  updated_at: string
}

export function listSalesPrices(params?: QueryParams): Promise<ApiResponse<SalesPrice[]>> {
  return request.get('/sales-prices', { params })
}

export function getSalesPrice(id: number): Promise<ApiResponse<SalesPrice>> {
  return request.get(`/sales-prices/${id}`)
}

export function createSalesPrice(data: Partial<SalesPrice>): Promise<ApiResponse<SalesPrice>> {
  return request.post('/sales-prices', data)
}

export function updateSalesPrice(
  id: number,
  data: Partial<SalesPrice>
): Promise<ApiResponse<SalesPrice>> {
  return request.put(`/sales-prices/${id}`, data)
}

export function approveSalesPrice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-prices/${id}/approve`)
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales-prices/history/${productId}`)
}

export function listPricingStrategies(): Promise<ApiResponse<any[]>> {
  return request.get('/sales-prices/strategies')
}
