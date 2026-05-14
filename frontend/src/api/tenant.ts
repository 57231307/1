import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface Tenant {
  id: number
  tenant_code: string
  tenant_name: string
  domain: string
  status: 'active' | 'inactive' | 'suspended'
  max_users: number
  current_users: number
  subscription_plan: string
  subscription_start_date: string
  subscription_end_date: string
  created_at: string
  updated_at: string
}

export function listTenants(params?: QueryParams): Promise<ApiResponse<Tenant[]>> {
  return request.get('/api/v1/erp/tenants', { params })
}

export function getTenant(id: number): Promise<ApiResponse<Tenant>> {
  return request.get(`/api/v1/erp/tenants/${id}`)
}

export function createTenant(data: Partial<Tenant>): Promise<ApiResponse<Tenant>> {
  return request.post('/api/v1/erp/tenants', data)
}

export function updateTenantStatus(id: number, data: { status: string }): Promise<ApiResponse<void>> {
  return request.put(`/api/v1/erp/tenants/${id}/status`, data)
}
