import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface SalesContract {
  id: number
  contract_no: string
  contract_name: string
  customer_id: number
  customer_name: string
  contract_type?: string
  contract_date: string
  signed_date?: string
  start_date: string
  end_date: string
  effective_date?: string
  expiry_date?: string
  total_amount: number
  currency: string
  payment_terms?: string
  payment_method?: string
  delivery_date?: string
  delivery_location?: string
  status: 'draft' | 'pending' | 'active' | 'completed' | 'cancelled'
  items: ContractItem[]
  return_items?: ContractItem[]
  delivery_terms: string
  remarks?: string
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

export function getSalesContractList(
  params?: QueryParams
): Promise<ApiResponse<{ list: SalesContract[]; total: number }>> {
  return request.get('/sales/sales-contracts', { params })
}

export function getSalesContract(id: number): Promise<ApiResponse<SalesContract>> {
  return request.get(`/sales/sales-contracts/${id}`)
}

export function createSalesContract(
  data: Partial<SalesContract>
): Promise<ApiResponse<SalesContract>> {
  return request.post('/sales/sales-contracts', data)
}

export function updateSalesContract(
  id: number,
  data: Partial<SalesContract>
): Promise<ApiResponse<SalesContract>> {
  return request.put(`/sales/sales-contracts/${id}`, data)
}

export function deleteSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales/sales-contracts/${id}`)
}

export function approveSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-contracts/${id}/approve`)
}

export function executeSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/sales/sales-contracts/${id}/execute`)
}

export function cancelSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.put(`/sales/sales-contracts/${id}/cancel`)
}
