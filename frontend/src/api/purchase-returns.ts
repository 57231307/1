import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface PurchaseReturn {
  id: number
  return_no: string
  purchase_order_id: number
  supplier_id: number
  supplier_name: string
  status: string
  total_amount: number
  remark: string
  created_at: string
  approved_at?: string
  executed_at?: string
}

export interface ReturnItem {
  id: number
  return_id: number
  product_id: number
  product_name: string
  quantity: number
  unit_price: number
  amount: number
  reason: string
}

export interface PurchaseReturnQueryParams extends QueryParams {
  supplier_id?: number
  purchase_order_id?: number
  status?: string
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

export function approvePurchaseReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchases/returns/${id}/approve`)
}

export function rejectPurchaseReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchases/returns/${id}/reject`)
}

export function executePurchaseReturn(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchases/returns/${id}/execute`)
}

export function getReturnItems(id: number): Promise<ApiResponse<ReturnItem[]>> {
  return request.get(`/purchases/returns/${id}/items`)
}

export function addReturnItem(id: number, data: Partial<ReturnItem>): Promise<ApiResponse<ReturnItem>> {
  return request.post(`/purchases/returns/${id}/items`, data)
}

export function updateReturnItem(itemId: number, data: Partial<ReturnItem>): Promise<ApiResponse<ReturnItem>> {
  return request.put(`/purchases/returns/items/${itemId}`, data)
}

export function deleteReturnItem(itemId: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchases/returns/items/${itemId}`)
}