import { request } from './request'

export interface QueryParams {
  page?: number
  pageSize?: number
  customerId?: number
  creditLevel?: string
  status?: string
}

export interface CustomerCredit {
  id?: number
  customerId?: number
  customerName?: string
  creditLevel?: string
  creditScore?: number
  creditLimit?: number
  creditDays?: number
  usedAmount?: number
  availableAmount?: number
  status?: string
  remark?: string
  createdAt?: string
  updatedAt?: string
}

export interface CreditRatingRequest {
  customerId: number
  creditLevel: string
  creditScore: number
  creditLimit: number
  creditDays: number
  remark?: string
}

export interface CreditLimitAdjustmentRequest {
  adjustmentType: string
  amount: number
  reason: string
}

export interface CreditAmountRequest {
  amount: number
}

export function listCredits(params?: QueryParams) {
  return request.get('/api/v1/customer-credit', { params })
}

export function getCredit(customerId: number) {
  return request.get(`/api/v1/customer-credit/${customerId}`)
}

export function createCredit(data: Partial<CustomerCredit>) {
  return request.post('/api/v1/customer-credit', data)
}

export function updateCredit(id: number, data: Partial<CustomerCredit>) {
  return request.put(`/api/v1/customer-credit/${id}`, data)
}

export function deleteCredit(id: number) {
  return request.delete(`/api/v1/customer-credit/${id}`)
}

export function setCreditRating(data: CreditRatingRequest) {
  return request.post('/api/v1/customer-credit/set-rating', data)
}

export function occupyCredit(customerId: number, data: CreditAmountRequest) {
  return request.post(`/api/v1/customer-credit/${customerId}/occupy`, data)
}

export function releaseCredit(customerId: number, data: CreditAmountRequest) {
  return request.post(`/api/v1/customer-credit/${customerId}/release`, data)
}

export function adjustCreditLimit(customerId: number, data: CreditLimitAdjustmentRequest) {
  return request.post(`/api/v1/customer-credit/${customerId}/adjust`, data)
}

export function deactivateCredit(customerId: number) {
  return request.post(`/api/v1/customer-credit/${customerId}/deactivate`)
}
