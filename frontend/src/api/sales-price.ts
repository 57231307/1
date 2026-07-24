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
  min_order_qty?: number
  price_type?: string
  price_level?: string
  effective_date: string
  expiry_date: string
  status: 'pending' | 'active' | 'expired' | 'inactive'
  remark: string
  created_at: string
  updated_at: string
}

export interface PricingStrategy {
  id: number
  name: string
  description: string
  type: 'tiered' | 'volume' | 'contract'
  rules: PricingStrategyRule[]
  status: 'active' | 'inactive'
  created_at: string
  updated_at: string
}

export interface PricingStrategyRule {
  id: number
  strategy_id: number
  min_quantity: number
  max_quantity?: number
  discount_rate: number
  price?: number
}

export function getSalesPriceList(
  params?: QueryParams
): Promise<ApiResponse<{ list: SalesPrice[]; total: number }>> {
  return request.get('/sales/sales-prices', { params })
}

export function getSalesPrice(id: number): Promise<ApiResponse<SalesPrice>> {
  return request.get(`/sales/sales-prices/${id}`)
}

export function createSalesPrice(data: Partial<SalesPrice>): Promise<ApiResponse<SalesPrice>> {
  return request.post('/sales/sales-prices', data)
}

export function updateSalesPrice(
  id: number,
  data: Partial<SalesPrice>
): Promise<ApiResponse<SalesPrice>> {
  return request.put(`/sales/sales-prices/${id}`, data)
}

export function deleteSalesPrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales/sales-prices/${id}`)
}

export function approveSalesPrice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-prices/${id}/approve`)
}

export function getPriceHistory(productId: number): Promise<ApiResponse<SalesPrice[]>> {
  return request.get(`/sales/sales-prices/history/${productId}`)
}

export function getPricingStrategyList(): Promise<ApiResponse<PricingStrategy[]>> {
  return request.get('/sales/sales-prices/strategies')
}
