import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface Tenant {
  id: number
  name: string
  code: string
  status: 'active' | 'suspended' | 'inactive'
  plan: 'basic' | 'standard' | 'enterprise'
  expireDate: string
  createdAt: string
}

export interface TenantConfig {
  theme: string
  language: string
  timezone: string
  features: Record<string, boolean>
}

export interface CreateTenantRequest {
  name: string
  code: string
  plan: 'basic' | 'standard' | 'enterprise'
  adminEmail: string
}

export interface UpdateTenantRequest {
  name?: string
  plan?: 'basic' | 'standard' | 'enterprise'
  expireDate?: string
}

export interface TenantQueryParams extends QueryParams {
  status?: string
  plan?: string
}

export function listTenants(params?: TenantQueryParams): Promise<ApiResponse<{ list: Tenant[]; total: number }>> {
  return request.get('/tenants', { params })
}

export function getTenant(id: number): Promise<ApiResponse<Tenant>> {
  return request.get(`/tenants/${id}`)
}

export function createTenant(data: CreateTenantRequest): Promise<ApiResponse<Tenant>> {
  return request.post('/tenants', data)
}

export function updateTenant(id: number, data: UpdateTenantRequest): Promise<ApiResponse<Tenant>> {
  return request.put(`/tenants/${id}`, data)
}

export function deleteTenant(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/tenants/${id}`)
}

export function activateTenant(id: number): Promise<ApiResponse<void>> {
  return request.post(`/tenants/${id}/activate`)
}

export function suspendTenant(id: number): Promise<ApiResponse<void>> {
  return request.post(`/tenants/${id}/suspend`)
}

export function getTenantConfig(id: number): Promise<ApiResponse<TenantConfig>> {
  return request.get(`/tenants/${id}/config`)
}

export function updateTenantConfig(id: number, data: Partial<TenantConfig>): Promise<ApiResponse<TenantConfig>> {
  return request.put(`/tenants/${id}/config`, data)
}
