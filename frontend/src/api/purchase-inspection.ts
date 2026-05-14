import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface QualityStandard {
  id: number
  standardCode: string
  standardName: string
  version: string
  type: 'product' | 'process'
  status: 'draft' | 'approved' | 'published'
  content: any
  attachments: string[]
  createdBy: number
  createdByName: string
  approvedBy?: number
  approvedByName?: string
  approvedAt?: string
  createdAt?: string
  updatedAt?: string
}

export interface QualityRecord {
  id: number
  recordNo: string
  inspectionType: string
  productId: number
  productName: string
  batchNo: string
  inspectionDate: string
  inspector: string
  result: 'pass' | 'fail' | 'pending'
  defects: Defect[]
  remark: string
  createdAt?: string
}

export interface Defect {
  id?: number
  recordId?: number
  defectType: string
  defectDescription: string
  severity: 'minor' | 'major' | 'critical'
  quantity: number
  processed: boolean
  processedBy?: string
  processedAt?: string
  remark?: string
}

export interface QualityRecordQueryParams extends QueryParams {
  productId?: number
  batchNo?: string
  result?: string
  inspectionType?: string
  inspectionDateStart?: string
  inspectionDateEnd?: string
}

export function listStandards(params?: { type?: string; status?: string }): Promise<ApiResponse<QualityStandard[]>> {
  return request.get('/quality-standards', { params })
}

export function getStandard(id: number): Promise<ApiResponse<QualityStandard>> {
  return request.get(`/quality-standards/${id}`)
}

export function createStandard(data: Partial<QualityStandard>): Promise<ApiResponse<QualityStandard>> {
  return request.post('/quality-standards', data)
}

export function updateStandard(id: number, data: Partial<QualityStandard>): Promise<ApiResponse<QualityStandard>> {
  return request.put(`/quality-standards/${id}`, data)
}

export function deleteStandard(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/quality-standards/${id}`)
}

export function approveStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/approve`)
}

export function publishStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/publish`)
}

export function getStandardVersions(id: number): Promise<ApiResponse<QualityStandard[]>> {
  return request.get(`/quality-standards/${id}/versions`)
}

export function listRecords(params?: QualityRecordQueryParams): Promise<ApiResponse<{ list: QualityRecord[]; total: number }>> {
  return request.get('/quality-inspection/records', { params })
}

export function getRecord(id: number): Promise<ApiResponse<QualityRecord>> {
  return request.get(`/quality-inspection/records/${id}`)
}

export function createRecord(data: Partial<QualityRecord>): Promise<ApiResponse<QualityRecord>> {
  return request.post('/quality-inspection/records', data)
}

export function listDefects(params?: { severity?: string; processed?: boolean }): Promise<ApiResponse<Defect[]>> {
  return request.get('/quality-inspection/defects', { params })
}

export function processDefect(id: number, data: { remark?: string }): Promise<ApiResponse<void>> {
  return request.post(`/quality-inspection/defects/${id}/process`, data)
}
