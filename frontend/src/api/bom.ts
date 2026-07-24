import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface Bom {
  id: number
  product_id: number
  product_name?: string
  product_code?: string
  version: string
  is_default: boolean
  status: 'draft' | 'active' | 'archived'
  remark?: string
  items?: BomItem[]
  created_at?: string
  updated_at?: string
}

export interface BomItem {
  id?: number
  bom_id?: number
  material_name: string
  quantity: number
  unit: string
  loss_rate: number
}

export interface BomQueryParams extends QueryParams {
  product_name?: string
  status?: string
}

// D14 Batch 5b：原 bomApi.list 转为风格 B 函数
export const getBomList = (params?: BomQueryParams) =>
  request.get<ApiResponse<{ list: Bom[]; total: number }>>('/boms', { params })

// D14 Batch 5b：原 bomApi.getById 转为风格 B 函数
export const getBomById = (id: number) => request.get<ApiResponse<Bom>>(`/boms/${id}`)

// D14 Batch 5b：原 bomApi.create 转为风格 B 函数
export const createBom = (data: Partial<Bom> & { items?: BomItem[] }) =>
  request.post<ApiResponse<Bom>>('/boms', data)

// D14 Batch 5b：原 bomApi.update 转为风格 B 函数
export const updateBom = (id: number, data: Partial<Bom> & { items?: BomItem[] }) =>
  request.put<ApiResponse<Bom>>(`/boms/${id}`, data)

// D14 Batch 5b：原 bomApi.delete 转为风格 B 函数
export const deleteBom = (id: number) => request.delete<ApiResponse<null>>(`/boms/${id}`)

// D14 Batch 5b：原 bomApi.copy 转为风格 B 函数
export const copyBom = (id: number) => request.post<ApiResponse<Bom>>(`/boms/${id}/copy`)

// D14 Batch 5b：原 bomApi.setDefault 转为风格 B 函数
export const setDefaultBom = (id: number) =>
  request.put<ApiResponse<Bom>>(`/boms/${id}/default`)

// D14 Batch 5b：原 bomApi.getVersions 转为风格 B 函数（获取BOM版本历史）
export const getBomVersionList = (productId: number) =>
  request.get<
    ApiResponse<{ id: number; version: string; created_at: string; is_default: boolean }[]>
  >(`/boms/product/${productId}/versions`)

// D14 Batch 5b：原 bomApi.submit 转为风格 B 函数（提交BOM审核）
export const submitBom = (id: number) => request.put<ApiResponse<void>>(`/boms/${id}/submit`)

// D14 Batch 5b：原 bomApi.approve 转为风格 B 函数（审核BOM）
export const approveBom = (id: number, data: { approved: boolean; remark?: string }) =>
  request.put<ApiResponse<void>>(`/boms/${id}/approve`, data)
