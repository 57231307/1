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

// D14 Batch 5b：原 productApi.list 转为风格 B 函数
export const getProductList = (params?: ProductQueryParams) =>
  request.get<ApiResponse<{ list: Product[]; total: number }>>('/products', { params })

// D14 Batch 5b：原 productApi.getById 转为风格 B 函数
export const getProductById = (id: number) => request.get<ApiResponse<Product>>(`/products/${id}`)

// D14 Batch 5b：原 productApi.create 转为风格 B 函数
export const createProduct = (data: Partial<Product>) =>
  request.post<ApiResponse<Product>>('/products', data)

// D14 Batch 5b：原 productApi.update 转为风格 B 函数
export const updateProduct = (id: number, data: Partial<Product>) =>
  request.put<ApiResponse<Product>>(`/products/${id}`, data)

// D14 Batch 5b：原 productApi.delete 转为风格 B 函数
export const deleteProduct = (id: number) => request.delete<ApiResponse<null>>(`/products/${id}`)

// D14 Batch 5b：原 productApi.batchCreate 转为风格 B 函数
export const batchCreateProducts = (data: Partial<Product>[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/create', data)

// D14 Batch 5b：原 productApi.batchUpdate 转为风格 B 函数
export const batchUpdateProducts = (data: Partial<Product>[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/update', data)

// D14 Batch 5b：原 productApi.batchDelete 转为风格 B 函数
export const batchDeleteProducts = (ids: number[]) =>
  request.post<ApiResponse<{ success: number; failed: number }>>('/products/batch/delete', {
    ids,
  })

// D14 Batch 5b：原 productApi.getCategories 转为风格 B 函数
export const getProductCategoryList = () =>
  request.get<ApiResponse<ProductCategory[]>>('/product-categories')

// D14 Batch 5b：原 productApi.createCategory 转为风格 B 函数
export const createProductCategory = (data: Partial<ProductCategory>) =>
  request.post<ApiResponse<ProductCategory>>('/product-categories', data)

// D14 Batch 5b：原 productApi.updateCategory 转为风格 B 函数
export const updateProductCategory = (id: number, data: Partial<ProductCategory>) =>
  request.put<ApiResponse<ProductCategory>>(`/product-categories/${id}`, data)

// D14 Batch 5b：原 productApi.deleteCategory 转为风格 B 函数
export const deleteProductCategory = (id: number) =>
  request.delete<ApiResponse<null>>(`/product-categories/${id}`)

// D14 Batch 5b：原 productApi.getCategoryTree 转为风格 B 函数
export const getProductCategoryTree = () =>
  request.get<ApiResponse<ProductCategory[]>>('/product-categories/tree')

// D14 Batch 5b：原 productApi.getColors 转为风格 B 函数
export const getProductColorList = (productId: number) =>
  request.get<ApiResponse<ProductColor[]>>(`/products/${productId}/colors`)

// D14 Batch 5b：原 productApi.createColor 转为风格 B 函数
export const createProductColor = (productId: number, data: Partial<ProductColor>) =>
  request.post<ApiResponse<ProductColor>>(`/products/${productId}/colors`, data)

// D14 Batch 5b：原 productApi.updateColor 转为风格 B 函数
export const updateProductColor = (productId: number, colorId: number, data: Partial<ProductColor>) =>
  request.put<ApiResponse<ProductColor>>(`/products/${productId}/colors/${colorId}`, data)

// D14 Batch 5b：原 productApi.deleteColor 转为风格 B 函数
export const deleteProductColor = (productId: number, colorId: number) =>
  request.delete<ApiResponse<null>>(`/products/${productId}/colors/${colorId}`)

// D14 Batch 5b：原 productApi.batchCreateColors 转为风格 B 函数
// P2-16 修复（批次 86 v2 复审）：批量创建颜色 ApiResponse<any> → ProductColor[]
export const batchCreateProductColors = (productId: number, colors: Partial<ProductColor>[]) =>
  request.post<ApiResponse<ProductColor[]>>(`/products/${productId}/colors/batch`, colors)

// D14 Batch 5b：原 productApi.getImportTemplate 转为风格 B 函数
export const getProductImportTemplate = () =>
  request.get<Blob>('/products/import-template', { responseType: 'blob' })

// D14 Batch 5b：原 productApi.importProducts 转为风格 B 函数
// P2-16 修复：导入结果 ApiResponse<any> → ProductImportResult
export const importProducts = (file: File) => {
  const formData = new FormData()
  formData.append('file', file)
  return request.post<ApiResponse<ProductImportResult>>('/products/import', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
}

// D14 Batch 5b：原 productApi.export 转为风格 B 函数
export const exportProducts = (params?: ProductQueryParams) =>
  request.get<Blob>('/products/export', { params, responseType: 'blob' })
