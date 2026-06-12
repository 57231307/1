import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface PurchasePrice {
  id: number
  product_id: number
  product_name: string
  product_code: string
  supplier_id: number
  supplier_name: string
  price: number
  currency: string
  unit: string
  min_order_qty?: number
  price_type?: string
  effective_date: string
  expiry_date: string
  status: 'active' | 'inactive'
  remark: string
  remarks?: string
  created_at: string
  updated_at: string
}

export function listPurchasePrices(
  params?: QueryParams
): Promise<ApiResponse<{ list: PurchasePrice[]; total: number }>> {
  return request.get('/purchase/purchase-prices', { params })
}

export function getPurchasePrice(id: number): Promise<ApiResponse<PurchasePrice>> {
  return request.get(`/purchase/purchase-prices/${id}`)
}

export function createPurchasePrice(
  data: Partial<PurchasePrice>
): Promise<ApiResponse<PurchasePrice>> {
  return request.post('/purchase/purchase-prices', data)
}

export function updatePurchasePrice(
  id: number,
  data: Partial<PurchasePrice>
): Promise<ApiResponse<PurchasePrice>> {
  return request.put(`/purchase/purchase-prices/${id}`, data)
}

export function deletePurchasePrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/purchase-prices/${id}`)
}

export function getPurchasePriceHistory(productId: number): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get(`/purchase/purchase-prices/history/${productId}`)
}
