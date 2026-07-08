import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface Warehouse {
  id: number
  warehouse_code: string
  warehouse_name: string
  warehouse_type: string
  address?: string
  contact_person?: string
  phone?: string
  capacity?: number
  status: string
  is_default?: boolean
  description?: string
  created_at?: string
}

export interface WarehouseLocation {
  id: number
  warehouse_id: number
  location_code: string
  location_type?: string
  max_weight?: number
  max_height?: number
  is_batch_managed?: boolean
  is_color_managed?: boolean
  created_at?: string
  updated_at?: string
}

export interface WarehouseQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  warehouse_type?: string
  status?: string
}

export const warehouseApi = {
  list: (params?: WarehouseQueryParams) =>
    request.get<ApiResponse<{ list: Warehouse[]; total: number }>>('/warehouses', { params }),

  getById: (id: number) => request.get<ApiResponse<Warehouse>>(`/warehouses/${id}`),

  create: (data: Partial<Warehouse>) => request.post<ApiResponse<Warehouse>>('/warehouses', data),

  update: (id: number, data: Partial<Warehouse>) =>
    request.put<ApiResponse<Warehouse>>(`/warehouses/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/warehouses/${id}`),

  getLocations: (warehouseId: number) =>
    request.get<ApiResponse<WarehouseLocation[]>>('/warehouses/locations', {
      params: { warehouse_id: warehouseId },
    }),

  createLocation: (data: Partial<WarehouseLocation>) =>
    request.post<ApiResponse<WarehouseLocation>>('/warehouses/locations', data),

  updateLocation: (id: number, data: Partial<WarehouseLocation>) =>
    request.put<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`, data),

  deleteLocation: (id: number) => request.delete<ApiResponse<null>>(`/warehouses/locations/${id}`),

  getLocation: (id: number) =>
    request.get<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`),
}
