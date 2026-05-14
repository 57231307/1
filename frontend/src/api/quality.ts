import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface QualityStandard {
  id: number
  standard_code: string
  standard_name: string
  version: string
  type: 'product' | 'process'
  status: 'draft' | 'approved' | 'published'
  content: string
  attachments: string[]
  created_by: number
  created_by_name: string
  approved_by: number
  approved_by_name: string
  approved_at: string
  created_at: string
  updated_at: string
}

export interface QualityRecord {
  id: number
  record_no: string
  inspection_type: string
  product_id: number
  product_name: string
  batch_no: string
  inspection_date: string
  inspector: string
  result: 'pass' | 'fail' | 'pending'
  defects: Defect[]
  remark: string
  created_at: string
}

export interface Defect {
  id: number
  record_id: number
  defect_type: string
  defect_description: string
  severity: 'minor' | 'major' | 'critical'
  quantity: number
  processed: boolean
  processed_by: string
  processed_at: string
  remark: string
}

export function listQualityStandards(params?: QueryParams): Promise<ApiResponse<QualityStandard[]>> {
  return request.get('/quality-standards', { params })
}

export function getQualityStandard(id: number): Promise<ApiResponse<QualityStandard>> {
  return request.get(`/quality-standards/${id}`)
}

export function createQualityStandard(data: Partial<QualityStandard>): Promise<ApiResponse<QualityStandard>> {
  return request.post('/quality-standards', data)
}

export function updateQualityStandard(id: number, data: Partial<QualityStandard>): Promise<ApiResponse<QualityStandard>> {
  return request.put(`/quality-standards/${id}`, data)
}

export function deleteQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/quality-standards/${id}`)
}

export function approveQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/approve`)
}

export function publishQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/publish`)
}

export function getQualityStandardVersions(id: number): Promise<ApiResponse<QualityStandard[]>> {
  return request.get(`/quality-standards/${id}/versions`)
}

export function listQualityRecords(params?: QueryParams): Promise<ApiResponse<QualityRecord[]>> {
  return request.get('/quality-inspection/records', { params })
}

export function getQualityRecord(id: number): Promise<ApiResponse<QualityRecord>> {
  return request.get(`/quality-inspection/records/${id}`)
}

export function createQualityRecord(data: Partial<QualityRecord>): Promise<ApiResponse<QualityRecord>> {
  return request.post('/quality-inspection/records', data)
}

export function listDefects(params?: QueryParams): Promise<ApiResponse<Defect[]>> {
  return request.get('/quality-inspection/defects', { params })
}

export function processDefect(id: number, data: { remark: string }): Promise<ApiResponse<void>> {
  return request.post(`/quality-inspection/defects/${id}/process`, data)
}
