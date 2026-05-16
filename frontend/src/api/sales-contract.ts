import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface SalesContract {
  id: number
  contract_no: string
  customer_id: number
  customer_name: string
  contract_date: string
  start_date: string
  end_date: string
  total_amount: number
  currency: string
  status: 'draft' | 'pending' | 'active' | 'completed' | 'cancelled'
  items: ContractItem[]
  return_items?: ContractItem[]
  payment_terms: string
  delivery_terms: string
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export interface ContractItem {
  id: number
  contract_id: number
  product_id: number
  product_name: string
  product_code: string
  quantity: number
  unit: string
  price: number
  amount: number
  remark: string
}

export function listSalesContracts(params?: QueryParams): Promise<ApiResponse<SalesContract[]>> {
  return request.get('/sales-contracts', { params })
}

export function getSalesContract(id: number): Promise<ApiResponse<SalesContract>> {
  return request.get(`/sales-contracts/${id}`)
}

export function createSalesContract(data: Partial<SalesContract>): Promise<ApiResponse<SalesContract>> {
  return request.post('/sales-contracts', data)
}

export function updateSalesContract(id: number, data: Partial<SalesContract>): Promise<ApiResponse<SalesContract>> {
  return request.put(`/sales-contracts/${id}`, data)
}

export function deleteSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales-contracts/${id}`)
}

export function approveSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales-contracts/${id}/approve`)
}

export function executeSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/sales-contracts/${id}/execute`)
}

export function cancelSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/sales-contracts/${id}/cancel`)
}
