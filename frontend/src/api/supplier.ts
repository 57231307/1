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

// D14 Batch 5b：原 supplierApi.list 与本函数 URL 同为 /purchase/suppliers，判定为重复，移除对象方法，保留本函数
export function getSupplierList(
  params?: SupplierQueryParams
): Promise<ApiResponse<{ list: Supplier[]; total: number }>> {
  return request.get('/purchase/suppliers', { params })
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

// D14 Batch 5b：原 supplierApi.getById 转为风格 B 函数
export const getSupplierById = (id: number) =>
  request.get<ApiResponse<Supplier>>(`/purchase/suppliers/${id}`)

// D14 Batch 5b：原 supplierApi.create 转为风格 B 函数
export const createSupplier = (data: Partial<Supplier>) =>
  request.post<ApiResponse<Supplier>>('/purchase/suppliers', data)

// D14 Batch 5b：原 supplierApi.update 转为风格 B 函数
export const updateSupplier = (id: number, data: Partial<Supplier>) =>
  request.put<ApiResponse<Supplier>>(`/purchase/suppliers/${id}`, data)

// D14 Batch 5b：原 supplierApi.delete 转为风格 B 函数
export const deleteSupplier = (id: number) =>
  request.delete<ApiResponse<null>>(`/purchase/suppliers/${id}`)

// D14 Batch 5b：原 supplierApi.evaluate 转为风格 B 函数
export const evaluateSupplier = (id: number, data: SupplierEvaluationData) =>
  request.post<ApiResponse<SupplierEvaluationResult>>(`/purchase/suppliers/${id}/evaluate`, data)

// D14 Batch 5b：原 supplierApi.getEvaluationHistory 转为风格 B 函数
export const getSupplierEvaluationHistory = (id: number) =>
  request.get<ApiResponse<SupplierEvaluationResult[]>>(`/purchase/suppliers/${id}/evaluations`)

// D14 Batch 5b：原 supplierApi.export 转为风格 B 函数
// V15 P0-S12 + P0-S15 新增（Batch 474）：带水印的 xlsx 导出
// 后端 GET /purchase/suppliers/export 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
// 水印已由后端注入（操作员/IP/时间戳），前端只需下载 Blob
export const exportSuppliers = (params?: SupplierQueryParams) =>
  request.get<Blob>('/purchase/suppliers/export', { params, responseType: 'blob' })
