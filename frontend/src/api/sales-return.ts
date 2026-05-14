import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface SalesReturn {
  id: number
  returnNo: string
  customerId: number
  customerName: string
  orderId: number
  orderNo: string
  returnDate: string
  totalAmount: number
  reason: string
  status: 'draft' | 'pending' | 'approved' | 'rejected' | 'completed'
  items: ReturnItem[]
  createdBy?: number
  createdByName?: string
  approvedBy?: number
  approvedByName?: string
  approvedAt?: string
  createdAt?: string
  updatedAt?: string
}

export interface ReturnItem {
  id?: number
  returnId?: number
  productId: number
  productName?: string
  productCode?: string
  quantity: number
  unit?: string
  price: number
  amount: number
  reason: string
}

export interface SalesReturnQueryParams extends QueryParams {
  customerId?: number
  orderId?: number
  status?: string
  returnDateStart?: string
  returnDateEnd?: string
}

export function listSalesReturns(params?: SalesReturnQueryParams): Promise<ApiResponse<{ list: SalesReturn[]; total: number }>> {
  return request.get('/sales-returns', { params })
}

export function getSalesReturn(id: number): Promise<ApiResponse<SalesReturn>> {
  return request.get(`/sales-returns/${id}`)
}

export function createSalesReturn(data: Partial<SalesReturn>): Promise<ApiResponse<SalesReturn>> {
  return request.post('/sales-returns', data)
}

export function updateSalesReturn(id: number, data: Partial<SalesReturn>): Promise<ApiResponse<SalesReturn>> {
  return request.put(`/sales-returns/${id}`, data)
}

export function submitSalesReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-returns/${id}/submit`)
}

export function approveSalesReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-returns/${id}/approve`)
}

export function rejectSalesReturn(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/sales-returns/${id}/reject`, { reason })
}

export function completeSalesReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-returns/${id}/complete`)
}
