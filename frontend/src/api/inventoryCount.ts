import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface InventoryCountEntity {
  id?: number
  count_no: string
  count_date: string
  warehouse_id: number
  warehouse_name?: string
  status: string
  created_at?: string
  created_by?: number
  created_by_name?: string
  completed_at?: string
}

export interface CountItem {
  id?: number
  count_id: number
  product_id: number
  product_code?: string
  product_name?: string
  color_no?: string
  grade?: string
  unit?: string
  system_qty: number
  actual_qty: number
  diff_qty: number
  cost_price: number
  diff_amount: number
  remark?: string
}

export const listInventoryCounts = (params?: any) => request.get('/inventory/counts', { params })

export const getInventoryCount = (id: number) => request.get(`/inventory/counts/${id}`)

export const createInventoryCount = (data: Partial<InventoryCountEntity>) =>
  request.post('/inventory/counts', data)

export const updateInventoryCount = (id: number, data: Partial<InventoryCountEntity>) =>
  request.put(`/inventory/counts/${id}`, data)

export const deleteInventoryCount = (id: number) => request.delete(`/inventory/counts/${id}`)

export const approveInventoryCount = (id: number) => request.post(`/inventory/counts/${id}/approve`)

export const completeInventoryCount = (id: number) =>
  request.post(`/inventory/counts/${id}/complete`)

export const getCountItems = (id: number) => request.get(`/inventory/counts/${id}`)

export const updateCountItem = (itemId: number, data: Partial<CountItem>) =>
  request.put(`/inventory/counts/items/${itemId}`, data)

export const deleteCountItem = (itemId: number) =>
  request.delete(`/inventory/counts/items/${itemId}`)

/**
 * 生成库存盘点单号
 * GET /inventory/counts/generate-no
 *
 * 单据号格式：`IC{yyyyMMdd}{4 位流水}`，例如 `IC202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generateInventoryCountNo = (): Promise<ApiResponse<{ count_no: string }>> =>
  request.get('/inventory/counts/generate-no')
