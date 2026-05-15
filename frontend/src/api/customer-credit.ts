import { request } from './request'
import type { ApiResponse } from './request'

export interface CustomerCredit {
  id?: number
  customerId?: number
  customerName?: string
  creditLimit?: number
  usedCredit?: number
  availableCredit?: number
  creditRating?: string
  status?: string
  validFrom?: string
  validTo?: string
  remarks?: string
  createdAt?: string
  updatedAt?: string
}

export interface CreditRating {
  rating: string
  creditLimit: number
  reason?: string
}

export interface CreditAdjustment {
  amount: number
  reason: string
  type: 'increase' | 'decrease'
}

export interface CreditOccupation {
  amount: number
  businessType: string
  businessId: number
  remarks?: string
}

export const customerCreditApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: CustomerCredit[]; total: number }>>('/customer-credits', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<CustomerCredit>>(`/customer-credits/${id}`),

  create: (data: Partial<CustomerCredit>) =>
    request.post<ApiResponse<CustomerCredit>>('/customer-credits', data),

  update: (id: number, data: Partial<CustomerCredit>) =>
    request.put<ApiResponse<CustomerCredit>>(`/customer-credits/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/customer-credits/${id}`),

  setCreditRating: (id: number, data: CreditRating) =>
    request.post<ApiResponse<CustomerCredit>>(`/customer-credits/${id}/rating`, data),

  occupyCredit: (id: number, data: CreditOccupation) =>
    request.post<ApiResponse<CustomerCredit>>(`/customer-credits/${id}/occupy`, data),

  releaseCredit: (id: number, occupationId: number) =>
    request.post<ApiResponse<CustomerCredit>>(`/customer-credits/${id}/release`, { occupationId }),

  adjustCreditLimit: (id: number, data: CreditAdjustment) =>
    request.post<ApiResponse<CustomerCredit>>(`/customer-credits/${id}/adjust`, data),

  deactivateCredit: (id: number) =>
    request.post<ApiResponse<CustomerCredit>>(`/customer-credits/${id}/deactivate`),
}
