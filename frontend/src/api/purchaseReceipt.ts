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
  return request.get<ApiResponse<{ list: PurchaseReceiptEntity[]; total: number }>>(
    '/purchase/receipts',
    { params }
  )
}

export function getPurchaseReceipt(id: number) {
  return request.get<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}`)
}

export function createPurchaseReceipt(data: Partial<PurchaseReceiptEntity>) {
  return request.post<ApiResponse<PurchaseReceiptEntity>>('/purchase/receipts', data)
}

export function updatePurchaseReceipt(id: number, data: Partial<PurchaseReceiptEntity>) {
  return request.put<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}`, data)
}

export function deletePurchaseReceipt(id: number) {
  return request.delete<ApiResponse<void>>(`/purchase/receipts/${id}`)
}

export function approvePurchaseReceipt(id: number) {
  return request.patch<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}/approve`)
}

// 入库单明细响应载荷
export interface ReceiptItemsResponse {
  items: ReceiptItem[]
  total?: number
}

/**
 * 获取入库单明细
 * 修复前返回 Promise<any>，导致调用方需要做大量类型断言；
 * 修复后返回明确的 ApiResponse<ReceiptItemsResponse>，调用方可直接 res.data?.items 解构。
 */
export function getReceiptItems(id: number) {
  return request.get<ApiResponse<ReceiptItemsResponse>>(`/purchase/receipts/${id}/items`)
}

export function addReceiptItem(id: number, data: Partial<ReceiptItem>) {
  return request.post<ApiResponse<ReceiptItem>>(`/purchase/receipts/${id}/items`, data)
}

export function updateReceiptItem(id: number, itemId: number, data: Partial<ReceiptItem>) {
  return request.put<ApiResponse<ReceiptItem>>(`/purchase/receipts/${id}/items/${itemId}`, data)
}

export function deleteReceiptItem(id: number, itemId: number) {
  return request.delete<ApiResponse<void>>(`/purchase/receipts/${id}/items/${itemId}`)
}

/**
 * 生成采购入库单号
 * GET /purchase/receipts/generate-no
 *
 * 单据号格式：`RK{yyyyMMdd}{4 位流水}`，例如 `RK202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generatePurchaseReceiptNo = (): Promise<ApiResponse<{ receipt_no: string }>> =>
  request.get('/purchase/receipts/generate-no')
