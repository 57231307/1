import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface EmailTemplate {
  id?: number
  name: string
  code: string
  subject_template: string
  body_template: string
  template_type: string
  description?: string
  is_active?: boolean
  created_at?: string
  updated_at?: string
}

export interface EmailLog {
  id?: number
  to: string
  cc?: string
  subject: string
  status: string
  template_id?: number
  sent_at?: string
  error_message?: string
}

export interface EmailStatistics {
  total_sent: number
  total_failed: number
  today_sent: number
  success_rate: number
}

export interface EmailQueryParams {
  page?: number
  page_size?: number
  status?: string
  template_id?: number
  start_date?: string
  end_date?: string
}

export interface SendEmailRequest {
  to: string | string[]
  cc?: string | string[]
  bcc?: string | string[]
  subject: string
  html_content?: string
  text_content?: string
  template_id?: number
  template_params?: Record<string, any>
}

export const emailApi = {
  // 发送邮件（后端路由 POST /api/v1/erp/send）
  send: (data: SendEmailRequest) =>
    request.post<ApiResponse<{ message_id: string; status: string; sent_at: string }>>(
      '/send',
      data
    ),

  // 邮件模板（后端路由 /api/v1/erp/email-templates）
  getTemplates: (params?: { page?: number; page_size?: number }) =>
    request.get<ApiResponse<{ list: EmailTemplate[]; total: number }>>('/email-templates', {
      params,
    }),

  getTemplateById: (id: number) =>
    request.get<ApiResponse<EmailTemplate>>(`/email-templates/${id}`),

  createTemplate: (data: Partial<EmailTemplate>) =>
    request.post<ApiResponse<EmailTemplate>>('/email-templates', data),

  updateTemplate: (id: number, data: Partial<EmailTemplate>) =>
    request.put<ApiResponse<EmailTemplate>>(`/email-templates/${id}`, data),

  deleteTemplate: (id: number) => request.delete<ApiResponse<void>>(`/email-templates/${id}`),

  // 发送记录（后端路由 /api/v1/erp/email-records）
  getRecords: (params?: EmailQueryParams) =>
    request.get<ApiResponse<{ list: EmailLog[]; total: number }>>('/email-records', { params }),

  // 发送统计（后端路由 /api/v1/erp/email-statistics）
  getStatistics: () => request.get<ApiResponse<EmailStatistics>>('/email-statistics'),
}
