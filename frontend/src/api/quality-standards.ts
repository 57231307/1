import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface QualityStandard {
  id: number
  standard_code: string
  standard_name: string
  version: string
  type: 'product' | 'process' | 'safety' | 'environmental'
  status: 'draft' | 'approved' | 'published' | 'archived'
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

export function listQualityStandards(
  params?: QueryParams
): Promise<ApiResponse<QualityStandard[]>> {
  return request.get('/quality-standards', { params })
}

export function getQualityStandard(id: number): Promise<ApiResponse<QualityStandard>> {
  return request.get(`/quality-standards/${id}`)
}

export function createQualityStandard(
  data: Partial<QualityStandard>
): Promise<ApiResponse<QualityStandard>> {
  return request.post('/quality-standards', data)
}

export function updateQualityStandard(
  id: number,
  data: Partial<QualityStandard>
): Promise<ApiResponse<QualityStandard>> {
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

export function archiveQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/archive`)
}
