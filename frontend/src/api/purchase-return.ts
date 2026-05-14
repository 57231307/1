import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface PurchaseReturn {
  id: number
  returnNo: string
  supplierId: number
  supplierName: string
  orderId?: number
  orderNo?: string
  receiptId?: number
  receiptNo?: string
  returnDate: string
  totalAmount: number
  reason: string
  status: 'draft' | 'pending' | 'approved' | 'rejected' | 'completed'
  items: PurchaseReturnItem[]
  createdBy?: number
  createdByName?: string
  approvedBy?: number
  approvedByName?: string
  approvedAt?: string
  createdAt?: string
  updatedAt?: string
}

export interface PurchaseReturnItem {
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

export interface PurchaseReturnQueryParams extends QueryParams {
  supplierId?: number
  orderId?: number
  status?: string
  returnDateStart?: string
  returnDateEnd?: string
}

export function listPurchaseReturns(params?: PurchaseReturnQueryParams): Promise<ApiResponse<{ list: PurchaseReturn[]; total: number }>> {
  return request.get('/purchases/returns', { params })
}

export function getPurchaseReturn(id: number): Promise<ApiResponse<PurchaseReturn>> {
  return request.get(`/purchases/returns/${id}`)
}

export function createPurchaseReturn(data: Partial<PurchaseReturn>): Promise<ApiResponse<PurchaseReturn>> {
  return request.post('/purchases/returns', data)
}

export function updatePurchaseReturn(id: number, data: Partial<PurchaseReturn>): Promise<ApiResponse<PurchaseReturn>> {
  return request.put(`/purchases/returns/${id}`, data)
}

export function deletePurchaseReturn(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchases/returns/${id}`)
}

export function submitPurchaseReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchases/returns/${id}/submit`)
}

export function approvePurchaseReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchases/returns/${id}/approve`)
}

export function rejectPurchaseReturn(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/purchases/returns/${id}/reject`, { reason })
}

export function listPurchaseReturnItems(returnId: number): Promise<ApiResponse<PurchaseReturnItem[]>> {
  return request.get(`/purchases/returns/${returnId}/items`)
}

export function createPurchaseReturnItem(returnId: number, data: Partial<PurchaseReturnItem>): Promise<ApiResponse<PurchaseReturnItem>> {
  return request.post(`/purchases/returns/${returnId}/items`, data)
}

export function updatePurchaseReturnItem(returnId: number, itemId: number, data: Partial<PurchaseReturnItem>): Promise<ApiResponse<PurchaseReturnItem>> {
  return request.put(`/purchases/returns/${returnId}/items/${itemId}`, data)
}

export function deletePurchaseReturnItem(returnId: number, itemId: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchases/returns/${returnId}/items/${itemId}`)
}
