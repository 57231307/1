import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface SupplierEvaluationQueryParams {
  page?: number
  pageSize?: number
  category?: string
  status?: string
  supplierId?: number
  period?: string
}

export interface EvaluationIndicator {
  id?: number
  indicatorCode?: string
  indicatorName?: string
  category?: string
  weight?: number
  maxScore?: number
  status?: string
  description?: string
  createdAt?: string
  updatedAt?: string
}

export interface EvaluationRecord {
  id?: number
  supplierId?: number
  supplierName?: string
  evaluationDate?: string
  period?: string
  totalScore?: number
  rating?: string
  status?: string
  evaluatorId?: number
  evaluatorName?: string
  remark?: string
  createdAt?: string
  updatedAt?: string
}

export interface SupplierScore {
  supplierId?: number
  supplierName?: string
  totalScore?: number
  rating?: string
  rank?: number
}

export interface CreateEvaluationIndicatorRequest {
  indicatorCode: string
  indicatorName: string
  category: string
  weight: number
  maxScore: number
  description?: string
}

export interface CreateEvaluationRequest {
  supplierId: number
  period: string
  remark?: string
}

export function listIndicators(
  params?: SupplierEvaluationQueryParams
): Promise<ApiResponse<{ list: EvaluationIndicator[]; total: number }>> {
  return request.get('/supplier-evaluation/evaluations/indicators', { params })
}

export function createIndicator(
  data: CreateEvaluationIndicatorRequest
): Promise<ApiResponse<EvaluationIndicator>> {
  return request.post('/supplier-evaluation/evaluations/indicators', data)
}

export function listEvaluationRecords(
  params?: SupplierEvaluationQueryParams
): Promise<ApiResponse<{ list: EvaluationRecord[]; total: number }>> {
  return request.get('/supplier-evaluation/evaluations/records', { params })
}

export function getEvaluationRecord(id: number): Promise<ApiResponse<EvaluationRecord>> {
  return request.get(`/supplier-evaluation/evaluations/records/${id}`)
}

export function createEvaluationRecord(
  data: CreateEvaluationRequest
): Promise<ApiResponse<EvaluationRecord>> {
  return request.post('/supplier-evaluation/evaluations/records', data)
}

export function getSupplierScore(supplierId: number): Promise<ApiResponse<SupplierScore>> {
  return request.get(`/supplier-evaluation/evaluations/suppliers/${supplierId}/score`)
}

export function getSupplierRankings(params?: {
  limit?: number
}): Promise<ApiResponse<SupplierScore[]>> {
  return request.get('/supplier-evaluation/evaluations/rankings', { params })
}

export function listEvaluations(
  params?: SupplierEvaluationQueryParams
): Promise<ApiResponse<{ list: EvaluationRecord[]; total: number }>> {
  return request.get('/supplier-evaluation/evaluations', { params })
}

export function getEvaluation(id: number): Promise<ApiResponse<EvaluationRecord>> {
  return request.get(`/supplier-evaluation/evaluations/${id}`)
}

export function createEvaluation(
  data: CreateEvaluationRequest
): Promise<ApiResponse<EvaluationRecord>> {
  return request.post('/supplier-evaluation/evaluations', data)
}

export function updateEvaluation(
  id: number,
  data: Partial<EvaluationRecord>
): Promise<ApiResponse<EvaluationRecord>> {
  return request.put(`/supplier-evaluation/evaluations/${id}`, data)
}

export function deleteEvaluation(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/supplier-evaluation/evaluations/${id}`)
}
