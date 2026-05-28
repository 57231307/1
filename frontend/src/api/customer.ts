import { request } from './request'
import type { ApiResponse } from './request'

export interface Customer {
  id: number
  customer_code: string
  customer_name: string
  contact_person?: string
  contact_phone?: string
  contact_email?: string
  address?: string
  city?: string
  province?: string
  country?: string
  postal_code?: string
  credit_limit?: number
  payment_terms?: number
  tax_id?: string
  bank_name?: string
  bank_account?: string
  customer_type?: string
  status: string
  notes?: string
  customer_industry?: string
  main_products?: string
  annual_purchase?: number
  quality_requirement?: string
  inspection_standard?: string
  created_at?: string
  updated_at?: string
}

export function listCustomers(
  params?: CustomerQueryParams
): Promise<ApiResponse<{ list: Customer[]; total: number }>> {
  return request.get('/customers', { params })
}

export interface CustomerQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  customer_type?: string
  status?: string
}

export const customerApi = {
  list: (params?: CustomerQueryParams) =>
    request.get<ApiResponse<{ list: Customer[]; total: number }>>('/customers', { params }),

  getById: (id: number) => request.get<ApiResponse<Customer>>(`/customers/${id}`),

  create: (data: Partial<Customer>) => request.post<ApiResponse<Customer>>('/customers', data),

  update: (id: number, data: Partial<Customer>) =>
    request.put<ApiResponse<Customer>>(`/customers/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/customers/${id}`),

  getCreditInfo: (id: number) =>
    request.get<ApiResponse<{ credit_limit: number; current_balance: number; available: number }>>(
      `/customers/${id}/credit`
    ),
}
