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
  effective_date: string
  expiry_date: string
  status: 'active' | 'inactive'
  remark: string
  created_at: string
  updated_at: string
}

export function listPurchasePrices(params?: QueryParams): Promise<ApiResponse<PurchasePrice[]>> {
  return request.get('/api/v1/erp/purchase-prices', { params })
}

export function getPurchasePrice(id: number): Promise<ApiResponse<PurchasePrice>> {
  return request.get(`/api/v1/erp/purchase-prices/${id}`)
}

export function createPurchasePrice(data: Partial<PurchasePrice>): Promise<ApiResponse<PurchasePrice>> {
  return request.post('/api/v1/erp/purchase-prices', data)
}

export function updatePurchasePrice(id: number, data: Partial<PurchasePrice>): Promise<ApiResponse<PurchasePrice>> {
  return request.put(`/api/v1/erp/purchase-prices/${id}`, data)
}

export function deletePurchasePrice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api/v1/erp/purchase-prices/${id}`)
}
