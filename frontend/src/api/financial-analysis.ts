import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface FinancialReport {
  id: number
  reportType: string
  reportName: string
  period: string
  periodStart?: string
  periodEnd?: string
  content?: any
  generatedBy: string
  status: string
  createdAt?: string
}

export interface FinancialIndicator {
  id: number
  indicatorCode: string
  indicatorName: string
  period: string
  currentValue: number
  previousValue?: number
  growthRate?: number
  targetValue?: number
  unit?: string
  createdAt?: string
}

export interface FinancialTrend {
  date: string
  value: number
}

export interface FinancialReportQueryParams extends QueryParams {
  reportType?: string
  period?: string
  periodStart?: string
  periodEnd?: string
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

export function executeReport(id: number): Promise<ApiResponse<FinancialReport>> {
  return request.post(`/financial-analysis/reports/${id}/execute`)
}

export function createIndicator(data: Partial<FinancialIndicator>): Promise<ApiResponse<FinancialIndicator>> {
  return request.post('/financial-analysis/indicators', data)
}

export function getTrends(params?: { period?: string; indicator?: string }): Promise<ApiResponse<FinancialTrend[]>> {
  return request.get('/financial-analysis/trends', { params })
}
