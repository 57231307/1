import { request } from './request'
import type { ApiResponse, PageResult } from './request'

export interface ReportQueryConfig {
  filters: Record<string, any>
  grouping: string[]
  sorting: { field: string; direction: 'asc' | 'desc' }[]
  aggregations: { field: string; function: string }[]
}

export interface ReportDisplayConfig {
  columns: { key: string; label: string; width?: number }[]
  chart_type: 'table' | 'bar' | 'line' | 'pie' | 'area' | 'none'
  formatting: Record<string, any>
}

export interface ReportTemplate {
  id: number
  name: string
  description: string
  type: string
  query_config: ReportQueryConfig
  display_config: ReportDisplayConfig
  created_at: string
  updated_at?: string
}

export interface ReportExecutionResult {
  id: number
  template_id: number
  template_name: string
  status: 'pending' | 'running' | 'success' | 'failed'
  data: any
  executed_at: string
  error_message?: string
}

export interface CreateReportTemplateRequest {
  name: string
  description?: string
  type: string
  query_config: ReportQueryConfig
  display_config: ReportDisplayConfig
}

export interface UpdateReportTemplateRequest {
  name?: string
  description?: string
  type?: string
  query_config?: ReportQueryConfig
  display_config?: ReportDisplayConfig
}

export interface ExecuteReportRequest {
  params?: Record<string, any>
}

export function listReportTemplates(params?: Record<string, any>): Promise<ApiResponse<PageResult<ReportTemplate>>> {
  return request.get('/reports/templates', { params })
}

export function getReportTemplate(id: number): Promise<ApiResponse<ReportTemplate>> {
  return request.get(`/reports/templates/${id}`)
}

export function createReportTemplate(data: CreateReportTemplateRequest): Promise<ApiResponse<ReportTemplate>> {
  return request.post('/reports/templates', data)
}

export function updateReportTemplate(id: number, data: UpdateReportTemplateRequest): Promise<ApiResponse<ReportTemplate>> {
  return request.put(`/reports/templates/${id}`, data)
}

export function deleteReportTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/reports/templates/${id}`)
}

export function executeReport(templateId: number, data?: ExecuteReportRequest): Promise<ApiResponse<ReportExecutionResult>> {
  return request.post(`/reports/execute/${templateId}`, data)
}

export function getReportHistory(params?: Record<string, any>): Promise<ApiResponse<PageResult<ReportExecutionResult>>> {
  return request.get('/reports/history', { params })
}

export function exportReport(reportId: number, format: 'pdf' | 'excel' = 'excel'): Promise<Blob> {
  return request.get(`/reports/export/${reportId}`, { params: { format }, responseType: 'blob' })
}
