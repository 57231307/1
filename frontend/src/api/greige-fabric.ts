import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface GreigeFabric {
  id: number
  fabric_code: string
  fabric_name: string
  fabric_type: string
  supplier_id: number
  supplier_name: string
  width: number
  weight: number
  unit: string
  composition: string
  quantity: number
  min_order_quantity: number
  status: 'active' | 'inactive'
  description: string
  created_at: string
  updated_at: string
}

export function listGreigeFabrics(params?: QueryParams): Promise<ApiResponse<GreigeFabric[]>> {
  return request.get('/production/greige-fabrics', { params })
}

export function getGreigeFabric(id: number): Promise<ApiResponse<GreigeFabric>> {
  return request.get(`/production/greige-fabrics/${id}`)
}

export function createGreigeFabric(
  data: Partial<GreigeFabric>
): Promise<ApiResponse<GreigeFabric>> {
  return request.post('/production/greige-fabrics', data)
}

export function updateGreigeFabric(
  id: number,
  data: Partial<GreigeFabric>
): Promise<ApiResponse<GreigeFabric>> {
  return request.put(`/production/greige-fabrics/${id}`, data)
}

export function deleteGreigeFabric(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/greige-fabrics/${id}`)
}

export function stockInGreigeFabric(
  id: number,
  data: { quantity: number; remark?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/production/greige-fabrics/${id}/stock-in`, data)
}

export function stockOutGreigeFabric(
  id: number,
  data: { quantity: number; remark?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/production/greige-fabrics/${id}/stock-out`, data)
}

export function getGreigeBySupplier(supplierId: number): Promise<ApiResponse<GreigeFabric[]>> {
  return request.get(`/production/greige-fabrics/by-supplier/${supplierId}`)
}
