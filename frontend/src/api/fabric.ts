import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface Fabric {
  id: number
  fabric_code: string
  fabric_name: string
  category_id?: number
  category_name?: string
  composition?: string
  weight?: string
  width?: string
  color?: string
  price?: number
  unit?: string
  supplier_id?: number
  supplier_name?: string
  stock_quantity?: number
  min_stock?: number
  image_url?: string
  description?: string
  is_active?: boolean
  created_at?: string
  updated_at?: string
}

export interface FabricCategory {
  id: number
  name: string
  code: string
  parent_id?: number
  sort_order?: number
}

export interface FabricQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  category_id?: number
  supplier_id?: number
  is_active?: boolean
}

// D14 Batch 5b：原 fabricApi.list 转为风格 B 函数
export const getFabricList = (params?: FabricQueryParams) =>
  request.get<ApiResponse<{ list: Fabric[]; total: number }>>('/products', { params })

// D14 Batch 5b：原 fabricApi.getById 转为风格 B 函数
export const getFabricById = (id: number) =>
  request.get<ApiResponse<Fabric>>(`/products/${id}`)

// D14 Batch 5b：原 fabricApi.create 转为风格 B 函数
export const createFabric = (data: Partial<Fabric>) =>
  request.post<ApiResponse<Fabric>>('/products', data)

// D14 Batch 5b：原 fabricApi.update 转为风格 B 函数
export const updateFabric = (id: number, data: Partial<Fabric>) =>
  request.put<ApiResponse<Fabric>>(`/products/${id}`, data)

// D14 Batch 5b：原 fabricApi.delete 转为风格 B 函数
export const deleteFabric = (id: number) =>
  request.delete<ApiResponse<null>>(`/products/${id}`)

// D14 Batch 5b：原 fabricApi.getCategories 转为风格 B 函数
export const getFabricCategoryList = () =>
  request.get<ApiResponse<FabricCategory[]>>('/product-categories')

// D14 Batch 5b：原 fabricApi.createCategory 转为风格 B 函数
export const createFabricCategory = (data: Partial<FabricCategory>) =>
  request.post<ApiResponse<FabricCategory>>('/product-categories', data)

// D14 Batch 5b：原 fabricApi.updateCategory 转为风格 B 函数
export const updateFabricCategory = (id: number, data: Partial<FabricCategory>) =>
  request.put<ApiResponse<FabricCategory>>(`/product-categories/${id}`, data)

// D14 Batch 5b：原 fabricApi.deleteCategory 转为风格 B 函数
export const deleteFabricCategory = (id: number) =>
  request.delete<ApiResponse<null>>(`/product-categories/${id}`)

// D14 Batch 5b：原 fabricApi.batchImport 转为风格 B 函数
export const batchImportFabrics = (data: Fabric[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/import', data)

// D14 Batch 5b：原 fabricApi.export 转为风格 B 函数
export const exportFabrics = (params?: FabricQueryParams) =>
  request.get<Blob>('/products/export', { params, responseType: 'blob' })
