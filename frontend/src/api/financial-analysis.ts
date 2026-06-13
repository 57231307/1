import { request } from './request'
import type { ApiResponse } from '@/types/api'

// 报表参数类型
export type ReportParameters = Record<string, string | number | boolean | null>

// 报表数据类型
export type ReportData = Record<string, string | number | boolean | null>

export interface FinancialReport {
  id?: number
  reportType: string
  reportName: string
  period?: string
  parameters?: ReportParameters
  status?: string
  createdBy?: number
  createdAt?: string
  updatedAt?: string
  executedAt?: string
  data?: ReportData
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
  parameters?: ReportParameters
}

// 报表查询参数
export interface ReportQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  reportType?: string
  status?: string
}

// 趋势查询参数
export interface TrendQueryParams {
  indicator?: string
  startDate?: string
  endDate?: string
}

export const listReports = (params?: ReportQueryParams) =>
  request.get('/financial-analysis/reports', { params })
export const createReport = (data: Partial<FinancialReport>) =>
  request.post('/financial-analysis/reports', data)
export const updateReport = (id: number, data: Partial<FinancialReport>) =>
  request.put(`/financial-analysis/reports/${id}`, data)
export const deleteReport = (id: number) => request.delete(`/financial-analysis/reports/${id}`)
export const executeFinancialReport = (id: number) =>
  request.post(`/financial-analysis/reports/${id}/execute`)

export const financialAnalysisApi = {
  listReports: (params?: ReportQueryParams) =>
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

  getTrends: (params?: TrendQueryParams) =>
    request.get<ApiResponse<{ trends: FinancialTrend[] }>>('/financial-analysis/trends', {
      params,
    }),
}
