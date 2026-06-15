import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface PurchaseReceiptEntity {
  id?: number
  receipt_no: string
  receipt_date: string
  purchase_order_id?: number
  purchase_order_no?: string
  supplier_id: number
  supplier_name?: string
  warehouse_id: number
  warehouse_name?: string
  status: string
  total_amount: number
  remark?: string
  created_at?: string
  created_by?: number
  created_by_name?: string
  approved_at?: string
  approved_by?: number
  approved_by_name?: string
  items?: ReceiptItem[]
}

export interface ReceiptItem {
  id?: number
  receipt_id?: number
  product_id: number
  product_code?: string
  product_name?: string
  color_no?: string
  grade?: string
  unit?: string
  quantity: number
  price: number
  amount: number
  remark?: string
}

export function listPurchaseReceipts(params?: any) {
  return request.get('/purchase/receipts', { params })
}

export function getPurchaseReceipt(id: number) {
  return request.get(`/purchase/receipts/${id}`)
}

export function createPurchaseReceipt(data: Partial<PurchaseReceiptEntity>) {
  return request.post('/purchase/receipts', data)
}

export function updatePurchaseReceipt(id: number, data: Partial<PurchaseReceiptEntity>) {
  return request.put(`/purchase/receipts/${id}`, data)
}

export function deletePurchaseReceipt(id: number) {
  return request.delete(`/purchase/receipts/${id}`)
}

export function approvePurchaseReceipt(id: number) {
  return request.patch(`/purchase/receipts/${id}/approve`)
}

export function getReceiptItems(id: number) {
  return request.get<ApiResponse<{ items: ReceiptItem[] }>>(`/purchase/receipts/${id}/items`)
}

export function addReceiptItem(id: number, data: Partial<ReceiptItem>) {
  return request.post(`/purchase/receipts/${id}/items`, data)
}

export function updateReceiptItem(id: number, itemId: number, data: Partial<ReceiptItem>) {
  return request.put(`/purchase/receipts/${id}/items/${itemId}`, data)
}

export function deleteReceiptItem(id: number, itemId: number) {
  return request.delete(`/purchase/receipts/${id}/items/${itemId}`)
}
