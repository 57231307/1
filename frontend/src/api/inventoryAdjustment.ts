import { request } from './request'

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

export interface QueryParams {
  page?: number
  pageSize?: number
  adjust_no?: string
  warehouse_id?: number
  status?: string
  adjust_date_start?: string
  adjust_date_end?: string
}

export function listInventoryAdjustments(params?: QueryParams) {
  return request.get('/inventory-adjustment', { params })
}

export function getInventoryAdjustment(id: number) {
  return request.get(`/inventory-adjustment/${id}`)
}

export function createInventoryAdjustment(data: Partial<InventoryAdjustmentEntity>) {
  return request.post('/inventory-adjustment', data)
}

export function updateInventoryAdjustment(id: number, data: Partial<InventoryAdjustmentEntity>) {
  return request.put(`/inventory-adjustment/${id}`, data)
}

export function deleteInventoryAdjustment(id: number) {
  return request.delete(`/inventory-adjustment/${id}`)
}

export function approveInventoryAdjustment(id: number) {
  return request.patch(`/inventory-adjustment/${id}/approve`)
}

export function getAdjustmentItems(id: number) {
  return request.get(`/inventory-adjustment/${id}/items`)
}

export function addAdjustmentItem(id: number, data: Partial<AdjustmentItem>) {
  return request.post(`/inventory-adjustment/${id}/items`, data)
}

export function updateAdjustmentItem(itemId: number, data: Partial<AdjustmentItem>) {
  return request.put(`/inventory-adjustment/items/${itemId}`, data)
}

export function deleteAdjustmentItem(itemId: number) {
  return request.delete(`/inventory-adjustment/items/${itemId}`)
}