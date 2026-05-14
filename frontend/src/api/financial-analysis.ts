import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface FinancialReport {
  id: number
  report_name: string
  report_type: string
  period: string
  status: 'draft' | 'pending' | 'completed'
  created_at?: string
  updated_at?: string
}

export function listFinancialReports(params?: QueryParams): Promise<ApiResponse<{ list: FinancialReport[]; total: number }>> {
  return request.get('/api/v1/erp/financial-analysis/reports', { params })
}

export function getFinancialReport(id: number): Promise<ApiResponse<FinancialReport>> {
  return request.get(`/api/v1/erp/financial-analysis/reports/${id}`)
}

export function createFinancialReport(data: Partial<FinancialReport>): Promise<ApiResponse<FinancialReport>> {
  return request.post('/api/v1/erp/financial-analysis/reports', data)
}

export function updateFinancialReport(id: number, data: Partial<FinancialReport>): Promise<ApiResponse<FinancialReport>> {
  return request.put(`/api/v1/erp/financial-analysis/reports/${id}`, data)
}

export function deleteFinancialReport(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api/v1/erp/financial-analysis/reports/${id}`)
}

export function executeFinancialReport(id: number): Promise<ApiResponse<any>> {
  return request.post(`/api/v1/erp/financial-analysis/reports/${id}/execute`)
}

export function getFinancialTrends(params?: { period?: string }): Promise<ApiResponse<any>> {
  return request.get('/api/v1/erp/financial-analysis/trends', { params })
}
