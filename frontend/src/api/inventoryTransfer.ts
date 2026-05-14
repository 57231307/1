import { request } from './request'

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
}

export interface TransferItem {
  id?: number
  transfer_id: number
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

export interface QueryParams {
  page?: number
  pageSize?: number
  transfer_no?: string
  from_warehouse_id?: number
  to_warehouse_id?: number
  status?: string
  transfer_date_start?: string
  transfer_date_end?: string
}

export function listInventoryTransfers(params?: QueryParams) {
  return request.get('/api/v1/inventory-transfer', { params })
}

export function getInventoryTransfer(id: number) {
  return request.get(`/api/v1/inventory-transfer/${id}`)
}

export function createInventoryTransfer(data: Partial<InventoryTransferEntity>) {
  return request.post('/api/v1/inventory-transfer', data)
}

export function updateInventoryTransfer(id: number, data: Partial<InventoryTransferEntity>) {
  return request.put(`/api/v1/inventory-transfer/${id}`, data)
}

export function deleteInventoryTransfer(id: number) {
  return request.delete(`/api/v1/inventory-transfer/${id}`)
}

export function approveInventoryTransfer(id: number) {
  return request.patch(`/api/v1/inventory-transfer/${id}/approve`)
}

export function getTransferItems(id: number) {
  return request.get(`/api/v1/inventory-transfer/${id}/items`)
}

export function addTransferItem(id: number, data: Partial<TransferItem>) {
  return request.post(`/api/v1/inventory-transfer/${id}/items`, data)
}

export function updateTransferItem(itemId: number, data: Partial<TransferItem>) {
  return request.put(`/api/v1/inventory-transfer/items/${itemId}`, data)
}

export function deleteTransferItem(itemId: number) {
  return request.delete(`/api/v1/inventory-transfer/items/${itemId}`)
}