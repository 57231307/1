import { request } from './request'

export interface QueryParams {
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

export function listIndicators(params?: QueryParams) {
  return request.get('/supplier-evaluation/indicators', { params })
}

export function createIndicator(data: CreateEvaluationIndicatorRequest) {
  return request.post('/supplier-evaluation/indicators', data)
}

export function listEvaluationRecords(params?: QueryParams) {
  return request.get('/supplier-evaluation/records', { params })
}

export function getEvaluationRecord(id: number) {
  return request.get(`/supplier-evaluation/records/${id}`)
}

export function createEvaluationRecord(data: CreateEvaluationRequest) {
  return request.post('/supplier-evaluation/records', data)
}

export function getSupplierScore(supplierId: number) {
  return request.get(`/supplier-evaluation/suppliers/${supplierId}/score`)
}

export function getSupplierRankings(params?: { limit?: number }) {
  return request.get('/supplier-evaluation/rankings', { params })
}

export function listEvaluations(params?: QueryParams) {
  return request.get('/supplier-evaluation/evaluations', { params })
}

export function getEvaluation(id: number) {
  return request.get(`/supplier-evaluation/evaluations/${id}`)
}

export function createEvaluation(data: CreateEvaluationRequest) {
  return request.post('/supplier-evaluation/evaluations', data)
}

export function updateEvaluation(id: number, data: Partial<EvaluationRecord>) {
  return request.put(`/supplier-evaluation/evaluations/${id}`, data)
}

export function deleteEvaluation(id: number) {
  return request.delete(`/supplier-evaluation/evaluations/${id}`)
}
