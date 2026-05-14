import { request } from './request'

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
}

export interface ReceiptItem {
  id?: number
  receipt_id: number
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

export interface QueryParams {
  page?: number
  pageSize?: number
  receipt_no?: string
  supplier_id?: number
  warehouse_id?: number
  status?: string
  receipt_date_start?: string
  receipt_date_end?: string
}

export function listPurchaseReceipts(params?: QueryParams) {
  return request.get('/api/v1/purchase-receipt', { params })
}

export function getPurchaseReceipt(id: number) {
  return request.get(`/api/v1/purchase-receipt/${id}`)
}

export function createPurchaseReceipt(data: Partial<PurchaseReceiptEntity>) {
  return request.post('/api/v1/purchase-receipt', data)
}

export function updatePurchaseReceipt(id: number, data: Partial<PurchaseReceiptEntity>) {
  return request.put(`/api/v1/purchase-receipt/${id}`, data)
}

export function deletePurchaseReceipt(id: number) {
  return request.delete(`/api/v1/purchase-receipt/${id}`)
}

export function approvePurchaseReceipt(id: number) {
  return request.patch(`/api/v1/purchase-receipt/${id}/approve`)
}

export function getReceiptItems(id: number) {
  return request.get(`/api/v1/purchase-receipt/${id}/items`)
}

export function addReceiptItem(id: number, data: Partial<ReceiptItem>) {
  return request.post(`/api/v1/purchase-receipt/${id}/items`, data)
}

export function updateReceiptItem(itemId: number, data: Partial<ReceiptItem>) {
  return request.put(`/api/v1/purchase-receipt/items/${itemId}`, data)
}

export function deleteReceiptItem(itemId: number) {
  return request.delete(`/api/v1/purchase-receipt/items/${itemId}`)
}