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
  template_params?: Record<string, unknown>
}

// D14 Batch 5b：原 emailApi.send 转为风格 B 函数（后端路由 POST /api/v1/erp/send）
export const sendEmail = (data: SendEmailRequest) =>
  request.post<ApiResponse<{ message_id: string; status: string; sent_at: string }>>(
    '/send',
    data
  )

// D14 Batch 5b：原 emailApi.getTemplates 转为风格 B 函数（后端路由 /api/v1/erp/email-templates）
export const getEmailTemplateList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<{ list: EmailTemplate[]; total: number }>>('/email-templates', {
    params,
  })

// D14 Batch 5b：原 emailApi.getTemplateById 转为风格 B 函数
export const getEmailTemplateById = (id: number) =>
  request.get<ApiResponse<EmailTemplate>>(`/email-templates/${id}`)

// D14 Batch 5b：原 emailApi.createTemplate 转为风格 B 函数
export const createEmailTemplate = (data: Partial<EmailTemplate>) =>
  request.post<ApiResponse<EmailTemplate>>('/email-templates', data)

// D14 Batch 5b：原 emailApi.updateTemplate 转为风格 B 函数
export const updateEmailTemplate = (id: number, data: Partial<EmailTemplate>) =>
  request.put<ApiResponse<EmailTemplate>>(`/email-templates/${id}`, data)

// D14 Batch 5b：原 emailApi.deleteTemplate 转为风格 B 函数
export const deleteEmailTemplate = (id: number) =>
  request.delete<ApiResponse<void>>(`/email-templates/${id}`)

// D14 Batch 5b：原 emailApi.getRecords 转为风格 B 函数（后端路由 /api/v1/erp/email-records）
export const getEmailRecordList = (params?: EmailQueryParams) =>
  request.get<ApiResponse<{ list: EmailLog[]; total: number }>>('/email-records', { params })

// D14 Batch 5b：原 emailApi.getStatistics 转为风格 B 函数（后端路由 /api/v1/erp/email-statistics）
export const getEmailStatistics = () =>
  request.get<ApiResponse<EmailStatistics>>('/email-statistics')
