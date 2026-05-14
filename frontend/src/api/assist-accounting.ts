import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface AssistDimension {
  id: number
  dimensionCode: string
  dimensionName: string
  dimensionType: string
  isActive: boolean
  createdAt?: string
}

export interface AssistRecord {
  id: number
  subjectId: number
  subjectCode: string
  subjectName: string
  dimensionId: number
  dimensionCode: string
  dimensionName: string
  dimensionValue: string
  amount: number
  direction: string
  businessId?: string
  businessType?: string
  period?: string
  createdAt?: string
}

export interface AssistSummary {
  subjectId: number
  subjectCode: string
  subjectName: string
  dimensionId: number
  dimensionCode: string
  dimensionName: string
  dimensionValue: string
  debitAmount: number
  creditAmount: number
  balance: number
}

export interface AssistRecordQueryParams extends QueryParams {
  subjectId?: number
  dimensionId?: number
  dimensionValue?: string
  businessType?: string
  period?: string
}

export function listDimensions(): Promise<ApiResponse<AssistDimension[]>> {
  return request.get('/assist-accounting/dimensions')
}

export function queryRecords(params?: AssistRecordQueryParams): Promise<ApiResponse<{ list: AssistRecord[]; total: number }>> {
  return request.get('/assist-accounting/records', { params })
}

export function getRecordsByBusiness(params?: { businessType?: string; businessId?: string }): Promise<ApiResponse<AssistRecord[]>> {
  return request.get('/assist-accounting/records/business', { params })
}

export function getRecordsByFiveDimension(fiveDimensionId: string): Promise<ApiResponse<AssistRecord[]>> {
  return request.get(`/assist-accounting/records/five-dimension/${encodeURIComponent(fiveDimensionId)}`)
}

export function getSummary(params?: { subjectId?: number; dimensionId?: number; period?: string }): Promise<ApiResponse<AssistSummary[]>> {
  return request.get('/assist-accounting/summary', { params })
}
