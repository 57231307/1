import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface Supplier {
  id: number
  supplier_code: string
  supplier_name: string
  supplier_short_name?: string
  supplier_type?: string
  credit_code?: string
  registered_address?: string
  business_address?: string
  legal_representative?: string
  registered_capital?: number
  establishment_date?: string
  business_term?: string
  business_scope?: string
  taxpayer_type?: string
  bank_name?: string
  bank_account?: string
  contact_phone?: string
  fax?: string
  website?: string
  email?: string
  main_business?: string
  main_market?: string
  employee_count?: number
  annual_revenue?: number
  grade?: string
  grade_score?: number
  last_evaluation_date?: string
  status: string
  is_enabled?: boolean
  assist_batch?: boolean
  assist_supplier?: boolean
  remarks?: string
  created_at?: string
  updated_at?: string
}

export function listSuppliers(
  params?: SupplierQueryParams
): Promise<ApiResponse<{ list: Supplier[]; total: number }>> {
  return request.get('/suppliers', { params })
}

export interface SupplierQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  category?: string
  grade?: string
  status?: string
}

export interface SupplierEvaluationData {
  score: number
  rating: string
  indicators?: Array<{
    indicator_id: number
    score: number
    remark?: string
  }>
  remark?: string
}

export interface SupplierEvaluationResult {
  id: number
  supplier_id: number
  score: number
  rating: string
  evaluation_date: string
  evaluator_id?: number
  evaluator_name?: string
  remark?: string
  created_at: string
}

export const supplierApi = {
  list: (params?: SupplierQueryParams) =>
    request.get<ApiResponse<{ list: Supplier[]; total: number }>>('/suppliers', { params }),

  getById: (id: number) => request.get<ApiResponse<Supplier>>(`/suppliers/${id}`),

  create: (data: Partial<Supplier>) => request.post<ApiResponse<Supplier>>('/suppliers', data),

  update: (id: number, data: Partial<Supplier>) =>
    request.put<ApiResponse<Supplier>>(`/suppliers/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/suppliers/${id}`),

  evaluate: (id: number, data: SupplierEvaluationData) =>
    request.post<ApiResponse<SupplierEvaluationResult>>(`/suppliers/${id}/evaluate`, data),

  getEvaluationHistory: (id: number) =>
    request.get<ApiResponse<SupplierEvaluationResult[]>>(`/suppliers/${id}/evaluations`),
}
