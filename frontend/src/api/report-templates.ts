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
  parameters: Record<string, unknown>
  is_system: boolean
  status: 'active' | 'inactive'
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export function getReportTemplateList(params?: QueryParams): Promise<ApiResponse<ReportTemplate[]>> {
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

// P2-16 修复（批次 86 v2 复审）：previewReportTemplate ApiResponse<any> → 显式接口

/** 报表模板预览结果 */
export interface ReportTemplatePreviewResult {
  template_id: number
  fields: string[]
  rows: Array<Record<string, unknown>>
  total: number
  [key: string]: unknown
}

export function previewReportTemplate(
  id: number,
  params?: Record<string, unknown>
): Promise<ApiResponse<ReportTemplatePreviewResult>> {
  return request.get(`/report-templates/${id}/preview`, { params })
}

export function generateReport(id: number, params: Record<string, unknown>): Promise<Blob> {
  return request.post(`/report-templates/${id}/generate`, params, {
    responseType: 'blob',
  })
}
