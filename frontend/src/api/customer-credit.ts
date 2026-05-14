import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface CustomerCredit {
  id: number
  customerId: number
  customerName?: string
  creditLimit: number
  usedAmount?: number
  availableAmount?: number
  creditRating?: string
  status: string
  createdAt?: string
  updatedAt?: string
}

export interface CustomerCreditQueryParams extends QueryParams {
  customerId?: number
  creditRating?: string
  status?: string
}

export interface CreditRatingRequest {
  customerId: number
  rating: string
  effectiveDate?: string
}

export interface CreditAmountRequest {
  amount: number
  orderId?: string
}

export interface CreditLimitAdjustmentRequest {
  newLimit: number
  reason: string
}

export function listCredits(params?: CustomerCreditQueryParams): Promise<ApiResponse<{ list: CustomerCredit[]; total: number }>> {
  return request.get('/customer-credits', { params })
}

export function getCredit(customerId: number): Promise<ApiResponse<CustomerCredit>> {
  return request.get(`/customer-credits/${customerId}`)
}

export function createCredit(data: Partial<CustomerCredit>): Promise<ApiResponse<CustomerCredit>> {
  return request.post('/customer-credits', data)
}

export function updateCredit(id: number, data: Partial<CustomerCredit>): Promise<ApiResponse<CustomerCredit>> {
  return request.put(`/customer-credits/${id}`, data)
}

export function deleteCredit(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/customer-credits/${id}`)
}

export function setCreditRating(data: CreditRatingRequest): Promise<ApiResponse<CustomerCredit>> {
  return request.post('/customer-credits/rating', data)
}

export function occupyCredit(customerId: number, data: CreditAmountRequest): Promise<ApiResponse<string>> {
  return request.post(`/customer-credits/${customerId}/occupy`, data)
}

export function releaseCredit(customerId: number, data: CreditAmountRequest): Promise<ApiResponse<string>> {
  return request.post(`/customer-credits/${customerId}/release`, data)
}

export function adjustCreditLimit(customerId: number, data: CreditLimitAdjustmentRequest): Promise<ApiResponse<string>> {
  return request.post(`/customer-credits/${customerId}/adjust`, data)
}

export function deactivateCredit(customerId: number): Promise<ApiResponse<string>> {
  return request.post(`/customer-credits/${customerId}/deactivate`)
}