import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface AssistDimension {
  id: number
  name: string
  code: string
  description?: string
}

export interface AssistRecord {
  id: number
  dimension_id: number
  dimension_name: string
  business_id: number
  business_type: string
  amount: number
  balance: number
  created_at: string
}

export interface AssistSummary {
  dimension_id: number
  dimension_name: string
  total_amount: number
  count: number
}

export interface AssistRecordQueryParams extends QueryParams {
  dimension_id?: number
  business_type?: string
  business_id?: number
}

export interface CreateAssistRecordParams {
  dimension_id: number
  business_id: number
  business_type: string
  amount: number
}

export interface UpdateAssistRecordParams {
  dimension_id?: number
  business_id?: number
  business_type?: string
  amount?: number
}

export function listAssistDimensions(): Promise<ApiResponse<AssistDimension[]>> {
  return request.get('/assist-accounting/dimensions')
}

export function queryAssistRecords(params?: AssistRecordQueryParams): Promise<ApiResponse<{ list: AssistRecord[]; total: number }>> {
  return request.get('/assist-accounting/records', { params })
}

export function getAssistRecordsByBusiness(params: { business_type: string; business_id: number }): Promise<ApiResponse<AssistRecord[]>> {
  return request.get('/assist-accounting/records/by-business', { params })
}

export function getAssistRecordsByFiveDimension(id: string): Promise<ApiResponse<AssistRecord[]>> {
  return request.get(`/assist-accounting/records/five-dimension/${id}`)
}

export function getAssistSummary(params?: { dimension_id?: number }): Promise<ApiResponse<AssistSummary[]>> {
  return request.get('/assist-accounting/summary', { params })
}

export function createAssistRecord(data: CreateAssistRecordParams): Promise<ApiResponse<AssistRecord>> {
  return request.post('/assist-accounting/records', data)
}

export function updateAssistRecord(id: number, data: UpdateAssistRecordParams): Promise<ApiResponse<AssistRecord>> {
  return request.put(`/assist-accounting/records/${id}`, data)
}

export function deleteAssistRecord(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/assist-accounting/records/${id}`)
}