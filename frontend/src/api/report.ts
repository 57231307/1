import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

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
