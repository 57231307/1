import { request } from './request'

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

export const listCustomerCredits = (params?: any) => request.get('/customer-credits', { params })

export const listCredits = listCustomerCredits

export const getCustomerCredit = (id: number) => request.get(`/customer-credits/${id}`)

export const createCustomerCredit = (data: Partial<CustomerCredit>) =>
  request.post('/customer-credits', data)

export const updateCustomerCredit = (id: number, data: Partial<CustomerCredit>) =>
  request.put(`/customer-credits/${id}`, data)

export const deleteCustomerCredit = (id: number) => request.delete(`/customer-credits/${id}`)

export const setCreditRating = (id: number, data: CreditRating) =>
  request.post(`/customer-credits/${id}/rating`, data)

export const occupyCredit = (id: number, data: CreditOccupation) =>
  request.post(`/customer-credits/${id}/occupy`, data)

export const releaseCredit = (id: number, occupation_id: number) =>
  request.post(`/customer-credits/${id}/release`, { occupation_id })

export const adjustCreditLimit = (id: number, data: CreditAdjustment) =>
  request.post(`/customer-credits/${id}/adjust`, data)

export const deactivateCredit = (id: number) => request.post(`/customer-credits/${id}/deactivate`)

export const evaluateCustomerCredit = (data: CreditEvaluationRequest & { id?: number }) =>
  request.post('/customer-credits/evaluate', data)
