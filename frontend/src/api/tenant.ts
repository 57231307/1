import { request } from './request'
import type { ApiResponse } from './request'

export interface Tenant {
  id?: number
  name: string
  code: string
  status?: 'active' | 'inactive' | 'suspended'
  contactName?: string
  contactEmail?: string
  contactPhone?: string
  address?: string
  licenseKey?: string
  expiresAt?: string
  maxUsers?: number
  currentUserCount?: number
  features?: string[]
  createdAt?: string
  updatedAt?: string
}

export interface TenantQueryParams {
  page?: number
  pageSize?: number
  status?: string
  keyword?: string
}

export const tenantApi = {
  list: (params?: TenantQueryParams) =>
    request.get<ApiResponse<{ list: Tenant[]; total: number }>>('/tenants', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<Tenant>>(`/tenants/${id}`),

  create: (data: Partial<Tenant>) =>
    request.post<ApiResponse<Tenant>>('/tenants', data),
}
