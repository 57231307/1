import { request } from './request'
import type { ApiResponse } from './request'

export interface Customer {
  id: number
  customer_code: string
  customer_name: string
  name: string
  contact_person?: string
  phone?: string
  email?: string
  address?: string
  credit_limit?: number
  current_balance?: number
  tax_number?: string
  bank_name?: string
  bank_account?: string
  customer_type?: string
  status: string
  created_at?: string
}

export function listCustomers(params?: CustomerQueryParams): Promise<ApiResponse<{ list: Customer[]; total: number }>> {
  return request.get('/api/v1/erp/customers', { params })
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

  create: (data: Partial<Customer>) =>
    request.post<ApiResponse<Customer>>('/customers', data),

  update: (id: number, data: Partial<Customer>) =>
    request.put<ApiResponse<Customer>>(`/customers/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<null>>(`/customers/${id}`),

  getCreditInfo: (id: number) =>
    request.get<ApiResponse<{ credit_limit: number; current_balance: number; available: number }>>(
      `/customers/${id}/credit`
    ),
}
