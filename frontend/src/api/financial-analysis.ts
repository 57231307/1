import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface FinancialReport {
  id?: number
  reportType: string
  reportName: string
  period?: string
  parameters?: Record<string, any>
  status?: string
  createdBy?: number
  createdAt?: string
  updatedAt?: string
  executedAt?: string
  data?: Record<string, any>
}

export interface FinancialIndicator {
  id?: number
  indicatorName: string
  formula: string
  category?: string
  unit?: string
  targetValue?: number
  actualValue?: number
  createdAt?: string
}

export interface FinancialTrend {
  date: string
  value: number
  indicator: string
}

export interface ReportExecutionRequest {
  reportId: number
  parameters?: Record<string, any>
}

export const listReports = (params?: any) => request.get('/financial-analysis/reports', { params })
export const createReport = (data: any) => request.post('/financial-analysis/reports', data)
export const updateReport = (id: number, data: any) =>
  request.put(`/financial-analysis/reports/${id}`, data)
export const deleteReport = (id: number) => request.delete(`/financial-analysis/reports/${id}`)
export const executeFinancialReport = (id: number) =>
  request.post(`/financial-analysis/reports/${id}/execute`)

export const financialAnalysisApi = {
  listReports: (params?: any) =>
    request.get<ApiResponse<{ list: FinancialReport[]; total: number }>>(
      '/financial-analysis/reports',
      { params }
    ),

  createReport: (data: Partial<FinancialReport>) =>
    request.post<ApiResponse<FinancialReport>>('/financial-analysis/reports', data),

  getReport: (id: number) =>
    request.get<ApiResponse<FinancialReport>>(`/financial-analysis/reports/${id}`),

  executeReport: (data: ReportExecutionRequest) =>
    request.post<ApiResponse<FinancialReport>>(
      `/financial-analysis/reports/${data.reportId}/execute`,
      data
    ),

  createIndicator: (data: Partial<FinancialIndicator>) =>
    request.post<ApiResponse<FinancialIndicator>>('/financial-analysis/indicators', data),

  getTrends: (params?: any) =>
    request.get<ApiResponse<{ trends: FinancialTrend[] }>>('/financial-analysis/trends', {
      params,
    }),
}
