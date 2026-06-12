import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface CustomerCredit {
  id?: number
  customer_id?: number
  customer_name?: string
  credit_limit?: number
  used_credit?: number
  available_credit?: number
  credit_rating?: string
  status?: string
  valid_from?: string
  valid_to?: string
  remarks?: string
  created_at?: string
  updated_at?: string
}

export interface CreditRating {
  rating: string
  credit_limit: number
  reason?: string
}

export interface CreditAdjustment {
  amount: number
  reason: string
  type: 'increase' | 'decrease'
}

export interface CreditOccupation {
  amount: number
  business_type: string
  business_id: number
  remarks?: string
}

export interface CreditEvaluationRequest {
  customer_id: number
  evaluation_date: string
}

export interface CustomerCreditQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  status?: string
  customer_id?: number
}

export const listCustomerCredits = (
  params?: CustomerCreditQueryParams
): Promise<ApiResponse<{ list: CustomerCredit[]; total: number }>> =>
  request.get('/crm/customer-credits', { params })

export const listCredits = listCustomerCredits

export const getCustomerCredit = (id: number): Promise<ApiResponse<CustomerCredit>> =>
  request.get(`/crm/customer-credits/${id}`)

export const createCustomerCredit = (
  data: Partial<CustomerCredit>
): Promise<ApiResponse<CustomerCredit>> => request.post('/crm/customer-credits', data)

export const updateCustomerCredit = (
  id: number,
  data: Partial<CustomerCredit>
): Promise<ApiResponse<CustomerCredit>> => request.put(`/crm/customer-credits/${id}`, data)

export const deleteCustomerCredit = (id: number): Promise<ApiResponse<void>> =>
  request.delete(`/crm/customer-credits/${id}`)

export const setCreditRating = (
  id: number,
  data: CreditRating
): Promise<ApiResponse<CustomerCredit>> => request.post(`/crm/customer-credits/${id}/rating`, data)

export const occupyCredit = (id: number, data: CreditOccupation): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${id}/occupy`, data)

export const releaseCredit = (id: number, occupation_id: number): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${id}/release`, { occupation_id })

export const adjustCreditLimit = (
  id: number,
  data: CreditAdjustment
): Promise<ApiResponse<CustomerCredit>> => request.post(`/crm/customer-credits/${id}/adjust`, data)

export const deactivateCredit = (id: number): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${id}/deactivate`)

export const evaluateCustomerCredit = (
  data: CreditEvaluationRequest & { id?: number }
): Promise<ApiResponse<CustomerCredit>> => request.post('/crm/customer-credits/evaluate', data)
