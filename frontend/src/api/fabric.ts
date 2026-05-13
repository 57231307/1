import { request } from './request'
import type { ApiResponse } from './request'

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

export const fabricApi = {
  list: (params?: FabricQueryParams) =>
    request.get<ApiResponse<{ list: Fabric[]; total: number }>>('/fabrics', { params }),

  getById: (id: number) => request.get<ApiResponse<Fabric>>(`/fabrics/${id}`),

  create: (data: Partial<Fabric>) =>
    request.post<ApiResponse<Fabric>>('/fabrics', data),

  update: (id: number, data: Partial<Fabric>) =>
    request.put<ApiResponse<Fabric>>(`/fabrics/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/fabrics/${id}`),

  getCategories: () =>
    request.get<ApiResponse<FabricCategory[]>>('/fabrics/categories'),

  createCategory: (data: Partial<FabricCategory>) =>
    request.post<ApiResponse<FabricCategory>>('/fabrics/categories', data),

  updateCategory: (id: number, data: Partial<FabricCategory>) =>
    request.put<ApiResponse<FabricCategory>>(`/fabrics/categories/${id}`, data),

  deleteCategory: (id: number) =>
    request.delete<ApiResponse<null>>(`/fabrics/categories/${id}`),

  batchImport: (data: Fabric[]) =>
    request.post<ApiResponse<{ success: number; failed: number }>>('/fabrics/batch-import', data),

  export: (params?: FabricQueryParams) =>
    request.get<Blob>('/fabrics/export', { params, responseType: 'blob' }),
}
