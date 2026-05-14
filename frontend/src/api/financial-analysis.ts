import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface FinancialReport {
  id: number
  reportName: string
  reportType: string
  description?: string
  params?: Record<string, any>
  period?: string
  status?: string
  createdAt: string
  updatedAt: string
}

export interface TrendData {
  period: string
  value: number
  comparisonValue?: number
}

export interface AnalysisSummary {
  totalRevenue?: number
  totalExpense?: number
  netProfit?: number
  revenueGrowthRate?: number
  expenseGrowthRate?: number
  profitGrowthRate?: number
  keyIndicators?: Record<string, number>
}

export interface FinancialReportQueryParams extends QueryParams {
  reportType?: string
}

export function listReports(params?: FinancialReportQueryParams): Promise<ApiResponse<{ list: FinancialReport[]; total: number }>> {
  return request.get('/financial-analysis/reports', { params })
}

export function getReport(id: number): Promise<ApiResponse<FinancialReport>> {
  return request.get(`/financial-analysis/reports/${id}`)
}

export function createReport(data: Partial<FinancialReport>): Promise<ApiResponse<FinancialReport>> {
  return request.post('/financial-analysis/reports', data)
}

export function updateReport(id: number, data: Partial<FinancialReport>): Promise<ApiResponse<FinancialReport>> {
  return request.put(`/financial-analysis/reports/${id}`, data)
}

export function deleteReport(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/financial-analysis/reports/${id}`)
}

export function executeFinancialReport(id: number): Promise<ApiResponse<FinancialReport>> {
  return request.post(`/financial-analysis/reports/${id}/execute`)
}

export function getAnalysisSummary(params?: { period?: string; periodType?: string }): Promise<ApiResponse<AnalysisSummary>> {
  return request.get('/financial-analysis/summary', { params })
}

export function getTrendAnalysis(params?: { period?: string; indicator?: string }): Promise<ApiResponse<TrendData[]>> {
  return request.get('/financial-analysis/trend', { params })
}