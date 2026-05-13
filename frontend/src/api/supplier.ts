import { request } from './request'
import type { ApiResponse } from './request'

export interface Supplier {
  id: number
  supplier_code: string
  supplier_name: string
  contact_person?: string
  phone?: string
  email?: string
  address?: string
  category?: string
  grade?: string
  status: string
  payment_terms?: string
  lead_time?: number
  created_at?: string
}

export interface SupplierQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  category?: string
  status?: string
}

export const supplierApi = {
  list: (params?: SupplierQueryParams) =>
    request.get<ApiResponse<{ list: Supplier[]; total: number }>>('/suppliers', { params }),

  getById: (id: number) => request.get<ApiResponse<Supplier>>(`/suppliers/${id}`),

  create: (data: Partial<Supplier>) =>
    request.post<ApiResponse<Supplier>>('/suppliers', data),

  update: (id: number, data: Partial<Supplier>) =>
    request.put<ApiResponse<Supplier>>(`/suppliers/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/suppliers/${id}`),

  evaluate: (id: number, data: any) =>
    request.post<ApiResponse<any>>(`/suppliers/${id}/evaluate`, data),

  getEvaluationHistory: (id: number) =>
    request.get<ApiResponse<any[]>>(`/suppliers/${id}/evaluations`),
}
