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
  // 发送邮件
  send: (data: SendEmailRequest) =>
    request.post<ApiResponse<{ message_id: string; status: string; sent_at: string }>>(
      '/emails/send',
      data
    ),

  // 邮件模板
  getTemplates: (params?: { page?: number; page_size?: number }) =>
    request.get<ApiResponse<{ list: EmailTemplate[]; total: number }>>('/emails/templates', {
      params,
    }),

  getTemplateById: (id: number) =>
    request.get<ApiResponse<EmailTemplate>>(`/emails/templates/${id}`),

  createTemplate: (data: Partial<EmailTemplate>) =>
    request.post<ApiResponse<EmailTemplate>>('/emails/templates', data),

  updateTemplate: (id: number, data: Partial<EmailTemplate>) =>
    request.put<ApiResponse<EmailTemplate>>(`/emails/templates/${id}`, data),

  deleteTemplate: (id: number) => request.delete<ApiResponse<void>>(`/emails/templates/${id}`),

  // 发送记录
  getRecords: (params?: EmailQueryParams) =>
    request.get<ApiResponse<{ list: EmailLog[]; total: number }>>('/emails/records', { params }),

  // 发送统计
  getStatistics: () => request.get<ApiResponse<EmailStatistics>>('/emails/statistics'),
}
