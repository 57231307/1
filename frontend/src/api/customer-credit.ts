import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface CustomerCredit {
  id: number
  customerId: number
  customerName?: string
  creditLevel?: string
  creditScore?: number
  creditLimit: number
  usedCredit?: number
  availableCredit?: number
  creditDays?: number
  status: string
  remark?: string
  createdAt?: string
  updatedAt?: string
}

export interface CustomerCreditQueryParams extends QueryParams {
  customerId?: number
  creditLevel?: string
  status?: string
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

export function listCredits(params?: CustomerCreditQueryParams): Promise<ApiResponse<{ list: CustomerCredit[]; total: number }>> {
  return request.get('/customer-credits', { params })
}

export function getCredit(customerId: number): Promise<ApiResponse<CustomerCredit>> {
  return request.get(`/customer-credits/${customerId}`)
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
