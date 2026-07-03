import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface Product {
  id: number
  product_code: string
  product_name: string
  category_id: number
  category_name?: string
  unit?: string
  price?: number
  cost_price?: number
  barcode?: string
  specification?: string
  description?: string
  is_active: boolean
  created_at?: string
  updated_at?: string
}

export interface ProductColor {
  id: number
  product_id: number
  color_code: string
  color_name: string
  rgb?: string
  price_adjustment?: number
  is_active: boolean
}

export interface ProductCategory {
  id: number
  name: string
  code: string
  parent_id?: number
  level?: number
  sort_order?: number
  children?: ProductCategory[]
}

export interface ProductQueryParams extends QueryParams {
  category_id?: number
  is_active?: boolean
  min_price?: number
  max_price?: number
}

/** 产品导入结果 */
export interface ProductImportResult {
  total: number
  success: number
  failed: number
  errors: Array<{ row: number; message: string }>
  [key: string]: unknown
}

export const productApi = {
  list: (params?: ProductQueryParams) =>
    request.get<ApiResponse<{ list: Product[]; total: number }>>('/products', { params }),

  getById: (id: number) => request.get<ApiResponse<Product>>(`/products/${id}`),

  create: (data: Partial<Product>) => request.post<ApiResponse<Product>>('/products', data),

  update: (id: number, data: Partial<Product>) =>
    request.put<ApiResponse<Product>>(`/products/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/products/${id}`),

  batchCreate: (data: Partial<Product>[]) =>
    request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/create', data),

  batchUpdate: (data: Partial<Product>[]) =>
    request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/update', data),

  batchDelete: (ids: number[]) =>
    request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/delete', {
      ids,
    }),

  getCategories: () => request.get<ApiResponse<ProductCategory[]>>('/product-categories'),

  createCategory: (data: Partial<ProductCategory>) =>
    request.post<ApiResponse<ProductCategory>>('/product-categories', data),

  updateCategory: (id: number, data: Partial<ProductCategory>) =>
    request.put<ApiResponse<ProductCategory>>(`/product-categories/${id}`, data),

  deleteCategory: (id: number) => request.delete<ApiResponse<null>>(`/product-categories/${id}`),

  getCategoryTree: () => request.get<ApiResponse<ProductCategory[]>>('/product-categories/tree'),

  getColors: (productId: number) =>
    request.get<ApiResponse<ProductColor[]>>(`/products/${productId}/colors`),

  createColor: (productId: number, data: Partial<ProductColor>) =>
    request.post<ApiResponse<ProductColor>>(`/products/${productId}/colors`, data),

  updateColor: (productId: number, colorId: number, data: Partial<ProductColor>) =>
    request.put<ApiResponse<ProductColor>>(`/products/${productId}/colors/${colorId}`, data),

  deleteColor: (productId: number, colorId: number) =>
    request.delete<ApiResponse<null>>(`/products/${productId}/colors/${colorId}`),

  // P2-16 修复（批次 86 v2 复审）：批量创建颜色 ApiResponse<any> → ProductColor[]
  batchCreateColors: (productId: number, colors: Partial<ProductColor>[]) =>
    request.post<ApiResponse<ProductColor[]>>(`/products/${productId}/colors/batch`, colors),

  getImportTemplate: () => request.get<Blob>('/products/import-template', { responseType: 'blob' }),

  // P2-16 修复：导入结果 ApiResponse<any> → ProductImportResult
  importProducts: (file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    return request.post<ApiResponse<ProductImportResult>>('/products/import', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
  },

  export: (params?: ProductQueryParams) =>
    request.get<Blob>('/products/export', { params, responseType: 'blob' }),
}
