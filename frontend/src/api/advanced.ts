import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

// AI 相关
export function forecastSales(params?: { period: string; product_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/forecast-sales', { params })
}

export function optimizeInventory(params?: { warehouse_id?: number }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/optimize-inventory', { params })
}

export function detectAnomalies(params?: { type: string }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/detect-anomalies', { params })
}

export function getRecommendations(): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/ai/recommendations')
}

// 报表相关
export interface ReportTemplate {
  id: number
  template_name: string
  template_code: string
  category: string
  description: string
  parameters: ReportParameter[]
  created_at: string
  updated_at: string
}

export interface ReportParameter {
  name: string
  type: string
  required: boolean
  default_value: any
  description: string
}

export function listReportTemplates(params?: QueryParams): Promise<ApiResponse<ReportTemplate[]>> {
  return request.get('/api/v1/erp/reports/templates', { params })
}

export function executeReport(templateCode: string, params?: Record<string, any>): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/reports/execute', { params: { template_code: templateCode, ...params } })
}

export function exportReport(templateCode: string, format: 'pdf' | 'excel', params?: Record<string, any>): Promise<Blob> {
  return request.get('/api/v1/erp/reports/export', { params: { template_code: templateCode, format, ...params }, responseType: 'blob' })
}

// 租户相关
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
