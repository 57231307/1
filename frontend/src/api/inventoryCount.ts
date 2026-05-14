import { request } from './request'

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

export interface QueryParams {
  page?: number
  pageSize?: number
  count_no?: string
  warehouse_id?: number
  status?: string
  count_date_start?: string
  count_date_end?: string
}

export function listInventoryCounts(params?: QueryParams) {
  return request.get('/api/v1/inventory-count', { params })
}

export function getInventoryCount(id: number) {
  return request.get(`/api/v1/inventory-count/${id}`)
}

export function createInventoryCount(data: Partial<InventoryCountEntity>) {
  return request.post('/api/v1/inventory-count', data)
}

export function updateInventoryCount(id: number, data: Partial<InventoryCountEntity>) {
  return request.put(`/api/v1/inventory-count/${id}`, data)
}

export function deleteInventoryCount(id: number) {
  return request.delete(`/api/v1/inventory-count/${id}`)
}

export function completeInventoryCount(id: number) {
  return request.patch(`/api/v1/inventory-count/${id}/complete`)
}

export function getCountItems(id: number) {
  return request.get(`/api/v1/inventory-count/${id}/items`)
}

export function updateCountItem(itemId: number, data: Partial<CountItem>) {
  return request.put(`/api/v1/inventory-count/items/${itemId}`, data)
}