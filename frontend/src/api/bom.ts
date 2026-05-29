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

export const bomApi = {
  list: (params?: BomQueryParams) =>
    request.get<ApiResponse<{ list: Bom[]; total: number }>>('/boms', { params }),

  getById: (id: number) => request.get<ApiResponse<Bom>>(`/boms/${id}`),

  create: (data: Partial<Bom> & { items?: BomItem[] }) =>
    request.post<ApiResponse<Bom>>('/boms', data),

  update: (id: number, data: Partial<Bom> & { items?: BomItem[] }) =>
    request.put<ApiResponse<Bom>>(`/boms/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/boms/${id}`),

  copy: (id: number) => request.post<ApiResponse<Bom>>(`/boms/${id}/copy`),

  setDefault: (id: number) => request.put<ApiResponse<Bom>>(`/boms/${id}/default`),
}
