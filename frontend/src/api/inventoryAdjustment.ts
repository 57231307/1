import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface InventoryAdjustmentEntity {
  id?: number
  adjust_no: string
  adjust_date: string
  warehouse_id: number
  warehouse_name?: string
  reason: string
  status: string
  total_amount: number
  created_at?: string
  created_by?: number
  created_by_name?: string
  approved_at?: string
  approved_by?: number
  approved_by_name?: string
  items?: AdjustmentItem[]
}

export interface AdjustmentItem {
  id?: number
  adjustment_id?: number
  product_id: number
  product_code?: string
  product_name?: string
  color_no?: string
  grade?: string
  unit?: string
  quantity: number
  cost_price: number
  amount: number
  remark?: string
}

// P2-9c 修复（批次 82 v1 复审）：库存调整列表查询参数强类型化
export interface InventoryAdjustmentQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  warehouse_id?: number
  adjustment_type?: string
  status?: string
}

export function listInventoryAdjustments(params?: InventoryAdjustmentQueryParams) {
  return request.get('/inventory/adjustments', { params })
}

export function getInventoryAdjustment(id: number) {
  return request.get(`/inventory/adjustments/${id}`)
}

export function createInventoryAdjustment(data: Partial<InventoryAdjustmentEntity>) {
  return request.post('/inventory/adjustments', data)
}

export function updateInventoryAdjustment(id: number, data: Partial<InventoryAdjustmentEntity>) {
  return request.put(`/inventory/adjustments/${id}`, data)
}

export function deleteInventoryAdjustment(id: number) {
  return request.delete(`/inventory/adjustments/${id}`)
}

export function approveInventoryAdjustment(id: number) {
  return request.post(`/inventory/adjustments/${id}/approve`)
}

export function rejectInventoryAdjustment(id: number) {
  return request.post(`/inventory/adjustments/${id}/reject`)
}

export function getAdjustmentItems(id: number) {
  return request.get(`/inventory/adjustments/${id}`)
}

export function addAdjustmentItem(id: number, data: Partial<AdjustmentItem>) {
  return request.post(`/inventory/adjustments/${id}`, data)
}

export function updateAdjustmentItem(itemId: number, data: Partial<AdjustmentItem>) {
  return request.put(`/inventory/adjustments/items/${itemId}`, data)
}

export function deleteAdjustmentItem(itemId: number) {
  return request.delete(`/inventory/adjustments/items/${itemId}`)
}

/**
 * 生成库存调整单号
 * GET /inventory/adjustments/generate-no
 *
 * 单据号格式：`IA{yyyyMMdd}{4 位流水}`，例如 `IA202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generateInventoryAdjustmentNo = (): Promise<ApiResponse<{ adjustment_no: string }>> =>
  request.get('/inventory/adjustments/generate-no')
