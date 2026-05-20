import { request } from './request'
import type { ApiResponse, PageResult } from './request'

export interface ReportField {
  key: string
  label: string
  type: 'string' | 'number' | 'date' | 'boolean'
  sortable: boolean
}

export interface ReportFilterCondition {
  field: string
  operator: 'eq' | 'ne' | 'gt' | 'lt' | 'gte' | 'lte' | 'contains' | 'in' | 'between'
  value: any
}

export interface ReportTemplateField {
  field_key: string
  display_label: string
  visible: boolean
  width?: number
  format?: string
}

export interface ReportTemplate {
  id: number
  name: string
  description: string
  type: string
  category: string
  fields: ReportTemplateField[]
  filters: ReportFilterCondition[]
  group_by: string[]
  sort_by: { field: string; direction: 'asc' | 'desc' }[]
  chart_type: 'none' | 'bar' | 'line' | 'pie' | 'area'
  is_system: boolean
  created_at: string
  updated_at: string
  created_by?: string
}

export interface ReportSubscription {
  id: number
  template_id: number
  template_name: string
  schedule: 'daily' | 'weekly' | 'monthly'
  schedule_time: string
  recipients: string[]
  format: 'pdf' | 'excel' | 'both'
  active: boolean
  created_at: string
  last_sent_at?: string
}

export interface CreateTemplateRequest {
  name: string
  description?: string
  type: string
  category: string
  fields: ReportTemplateField[]
  filters?: ReportFilterCondition[]
  group_by?: string[]
  sort_by?: { field: string; direction: 'asc' | 'desc' }[]
  chart_type?: string
}

export interface UpdateTemplateRequest {
  name?: string
  description?: string
  fields?: ReportTemplateField[]
  filters?: ReportFilterCondition[]
  group_by?: string[]
  sort_by?: { field: string; direction: 'asc' | 'desc' }[]
  chart_type?: string
}

export interface CreateSubscriptionRequest {
  template_id: number
  schedule: 'daily' | 'weekly' | 'monthly'
  schedule_time: string
  recipients: string[]
  format: 'pdf' | 'excel' | 'both'
}

export interface UpdateSubscriptionRequest {
  schedule?: 'daily' | 'weekly' | 'monthly'
  schedule_time?: string
  recipients?: string[]
  format?: 'pdf' | 'excel' | 'both'
  active?: boolean
}

export function listReportTemplates(params?: Record<string, any>): Promise<ApiResponse<PageResult<ReportTemplate>>> {
  return request.get('/report/templates', { params })
}

export function getReportTemplate(id: number): Promise<ApiResponse<ReportTemplate>> {
  return request.get(`/report/templates/${id}`)
}

export function createReportTemplate(data: CreateTemplateRequest): Promise<ApiResponse<ReportTemplate>> {
  return request.post('/report/templates', data)
}

export function updateReportTemplate(id: number, data: UpdateTemplateRequest): Promise<ApiResponse<ReportTemplate>> {
  return request.put(`/report/templates/${id}`, data)
}

export function deleteReportTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/report/templates/${id}`)
}

export function getAvailableFields(templateType: string): Promise<ApiResponse<ReportField[]>> {
  return request.get(`/report/fields/${templateType}`)
}

export function exportReport(templateId: number, params: { format: 'pdf' | 'excel'; date_range?: { start: string; end: string }; filters?: ReportFilterCondition[] }): Promise<Blob> {
  return request.post(`/report/templates/${templateId}/export`, params, { responseType: 'blob' })
}

export function previewReport(templateId: number, params?: Record<string, any>): Promise<ApiResponse<any>> {
  return request.get(`/report/templates/${templateId}/preview`, { params })
}

export function listSubscriptions(params?: Record<string, any>): Promise<ApiResponse<PageResult<ReportSubscription>>> {
  return request.get('/report/subscriptions', { params })
}

export function createSubscription(data: CreateSubscriptionRequest): Promise<ApiResponse<ReportSubscription>> {
  return request.post('/report/subscriptions', data)
}

export function updateSubscription(id: number, data: UpdateSubscriptionRequest): Promise<ApiResponse<ReportSubscription>> {
  return request.put(`/report/subscriptions/${id}`, data)
}

export function deleteSubscription(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/report/subscriptions/${id}`)
}

export function toggleSubscription(id: number): Promise<ApiResponse<ReportSubscription>> {
  return request.put(`/report/subscriptions/${id}/toggle`)
}

export function sendSubscriptionNow(id: number): Promise<ApiResponse<{ message: string }>> {
  return request.post(`/report/subscriptions/${id}/send`)
}
