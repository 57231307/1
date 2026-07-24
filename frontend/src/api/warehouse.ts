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

// D14 Batch 5b：原 warehouseApi.list 转为风格 B 函数
export const getWarehouseList = (params?: WarehouseQueryParams) =>
  request.get<ApiResponse<{ list: Warehouse[]; total: number }>>('/warehouses', { params })

// D14 Batch 5b：原 warehouseApi.getById 转为风格 B 函数
export const getWarehouseById = (id: number) =>
  request.get<ApiResponse<Warehouse>>(`/warehouses/${id}`)

// D14 Batch 5b：原 warehouseApi.create 转为风格 B 函数
export const createWarehouse = (data: Partial<Warehouse>) =>
  request.post<ApiResponse<Warehouse>>('/warehouses', data)

// D14 Batch 5b：原 warehouseApi.update 转为风格 B 函数
export const updateWarehouse = (id: number, data: Partial<Warehouse>) =>
  request.put<ApiResponse<Warehouse>>(`/warehouses/${id}`, data)

// D14 Batch 5b：原 warehouseApi.delete 转为风格 B 函数
export const deleteWarehouse = (id: number) =>
  request.delete<ApiResponse<null>>(`/warehouses/${id}`)

// D14 Batch 5b：原 warehouseApi.getLocations 转为风格 B 函数
export const getWarehouseLocationList = (warehouseId: number) =>
  request.get<ApiResponse<WarehouseLocation[]>>('/warehouses/locations', {
    params: { warehouse_id: warehouseId },
  })

// D14 Batch 5b：原 warehouseApi.createLocation 转为风格 B 函数
export const createWarehouseLocation = (data: Partial<WarehouseLocation>) =>
  request.post<ApiResponse<WarehouseLocation>>('/warehouses/locations', data)

// D14 Batch 5b：原 warehouseApi.updateLocation 转为风格 B 函数
export const updateWarehouseLocation = (id: number, data: Partial<WarehouseLocation>) =>
  request.put<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`, data)

// D14 Batch 5b：原 warehouseApi.deleteLocation 转为风格 B 函数
export const deleteWarehouseLocation = (id: number) =>
  request.delete<ApiResponse<null>>(`/warehouses/locations/${id}`)

// D14 Batch 5b：原 warehouseApi.getLocation 转为风格 B 函数
export const getWarehouseLocation = (id: number) =>
  request.get<ApiResponse<WarehouseLocation>>(`/warehouses/locations/${id}`)
