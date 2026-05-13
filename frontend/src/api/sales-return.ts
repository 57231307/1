import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface SalesReturn {
  id: number
  return_no: string
  customer_id: number
  customer_name: string
  order_id: number
  order_no: string
  return_date: string
  total_amount: number
  reason: string
  status: 'draft' | 'pending' | 'approved' | 'rejected' | 'completed'
  items: ReturnItem[]
  created_by: number
  created_by_name: string
  approved_by: number
  approved_by_name: string
  approved_at: string
  created_at: string
  updated_at: string
}

export interface ReturnItem {
  id: number
  return_id: number
  product_id: number
  product_name: string
  product_code: string
  quantity: number
  unit: string
  price: number
  amount: number
  reason: string
}

export function listSalesReturns(params?: QueryParams): Promise<ApiResponse<SalesReturn[]>> {
  return request.get('/api/v1/erp/sales-returns', { params })
}

export function getSalesReturn(id: number): Promise<ApiResponse<SalesReturn>> {
  return request.get(`/api/v1/erp/sales-returns/${id}`)
}

export function createSalesReturn(data: Partial<SalesReturn>): Promise<ApiResponse<SalesReturn>> {
  return request.post('/api/v1/erp/sales-returns', data)
}
