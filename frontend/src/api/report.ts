import { request } from './request'
import type { ApiResponse } from './request'

export interface ReportDefinition {
  id?: number
  reportName: string
  reportCode: string
  reportType: string
  template?: string
  parameters?: ReportParameter[]
  columns?: ReportColumn[]
  filters?: ReportFilter[]
  isActive?: boolean
  createdBy?: number
  createdAt?: string
  updatedAt?: string
}

export interface ReportParameter {
  name: string
  label: string
  type: string
  defaultValue?: string
  required?: boolean
  options?: string[]
}

export interface ReportColumn {
  field: string
  label: string
  width?: number
  format?: string
  visible?: boolean
}

export const reportApi = {
  list: (params?: any) =>
    request.get<ApiResponse<{ list: ReportDefinition[]; total: number }>>('/reports', { params }),

  execute: (reportCode: string, parameters?: Record<string, any>) =>
    request.post<ApiResponse<{ data: any[] }>>(`/reports/${reportCode}/execute`, parameters),

  export: (reportCode: string, parameters?: Record<string, any>, format?: string) =>
    request.post<ApiResponse<{ url: string }>>(`/reports/${reportCode}/export`, { ...parameters, format }),
}
