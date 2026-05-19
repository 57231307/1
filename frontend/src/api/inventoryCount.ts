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

export const listInventoryCounts = (params?: QueryParams) =>
  request.get('/inventory/counts', { params })

export const getInventoryCount = (id: number) =>
  request.get(`/inventory/counts/${id}`)

export const createInventoryCount = (data: Partial<InventoryCountEntity>) =>
  request.post('/inventory/counts', data)

export const updateInventoryCount = (id: number, data: Partial<InventoryCountEntity>) =>
  request.put(`/inventory/counts/${id}`, data)

export const deleteInventoryCount = (id: number) =>
  request.delete(`/inventory/counts/${id}`)

export const approveInventoryCount = (id: number) =>
  request.post(`/inventory/counts/${id}/approve`)

export const completeInventoryCount = (id: number) =>
  request.post(`/inventory/counts/${id}/complete`)

export const getCountItems = (id: number) =>
  request.get(`/inventory/counts/${id}`)

export const updateCountItem = (itemId: number, data: Partial<CountItem>) =>
  request.put(`/inventory/counts/items/${itemId}`, data)
