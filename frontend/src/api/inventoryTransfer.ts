import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface InventoryTransferEntity {
  id?: number
  transfer_no: string
  transfer_date: string
  from_warehouse_id: number
  from_warehouse_name?: string
  to_warehouse_id: number
  to_warehouse_name?: string
  status: string
  total_amount: number
  created_at?: string
  created_by?: number
  created_by_name?: string
  approved_at?: string
  approved_by?: number
  approved_by_name?: string
  items?: TransferItem[]
}

export interface TransferItem {
  id?: number
  transfer_id?: number
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

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function listInventoryTransfers(params?: any) {
  return request.get('/inventory/transfers', { params })
}

export function getInventoryTransfer(id: number) {
  return request.get(`/inventory/transfers/${id}`)
}

export function createInventoryTransfer(data: Partial<InventoryTransferEntity>) {
  return request.post('/inventory/transfers', data)
}

export function updateInventoryTransfer(id: number, data: Partial<InventoryTransferEntity>) {
  return request.put(`/inventory/transfers/${id}`, data)
}

export function deleteInventoryTransfer(id: number) {
  return request.delete(`/inventory/transfers/${id}`)
}

export function approveInventoryTransfer(id: number) {
  return request.post(`/inventory/transfers/${id}/approve`)
}

export function getTransferItems(id: number) {
  return request.get(`/inventory/transfers/${id}`)
}

export function addTransferItem(id: number, data: Partial<TransferItem>) {
  return request.post(`/inventory/transfers/${id}`, data)
}

export function updateTransferItem(itemId: number, data: Partial<TransferItem>) {
  return request.put(`/inventory/transfers/items/${itemId}`, data)
}

export function deleteTransferItem(itemId: number) {
  return request.delete(`/inventory/transfers/items/${itemId}`)
}

/**
 * 生成库存调拨单号
 * GET /inventory/transfers/generate-no
 *
 * 单据号格式：`IT{yyyyMMdd}{4 位流水}`，例如 `IT202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generateInventoryTransferNo = (): Promise<ApiResponse<{ transfer_no: string }>> =>
  request.get('/inventory/transfers/generate-no')
