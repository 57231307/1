import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface ReportTemplate {
  id: number
  template_code: string
  template_name: string
  description: string
  category: 'sales' | 'inventory' | 'finance' | 'production' | 'custom'
  format: 'pdf' | 'excel' | 'word' | 'html'
  content: string
  parameters: Record<string, any>
  is_system: boolean
  status: 'active' | 'inactive'
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export function listReportTemplates(params?: QueryParams): Promise<ApiResponse<ReportTemplate[]>> {
  return request.get('/report-templates', { params })
}

export function getReportTemplate(id: number): Promise<ApiResponse<ReportTemplate>> {
  return request.get(`/report-templates/${id}`)
}

export function createReportTemplate(
  data: Partial<ReportTemplate>
): Promise<ApiResponse<ReportTemplate>> {
  return request.post('/report-templates', data)
}

export function updateReportTemplate(
  id: number,
  data: Partial<ReportTemplate>
): Promise<ApiResponse<ReportTemplate>> {
  return request.put(`/report-templates/${id}`, data)
}

export function deleteReportTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/report-templates/${id}`)
}

export function previewReportTemplate(
  id: number,
  params?: Record<string, any>
): Promise<ApiResponse<any>> {
  return request.get(`/report-templates/${id}/preview`, { params })
}

export function generateReport(id: number, params: Record<string, any>): Promise<Blob> {
  return request.post(`/report-templates/${id}/generate`, params, {
    responseType: 'blob',
  })
}
