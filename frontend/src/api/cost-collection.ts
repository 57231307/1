import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface CostCollection {
  id: number
  collectionNo: string
  collectionType: string
  productId?: number
  productName?: string
  batchNo?: string
  periodStart: string
  periodEnd: string
  totalAmount: number
  detail?: any
  status: string
  remark?: string
  createdBy?: string
  createdAt?: string
  updatedAt?: string
}

export interface CostItem {
  id: number
  collectionId: number
  costType: string
  costName: string
  amount: number
  unit?: string
  quantity?: number
  remark?: string
}

export interface CostAnalysisSummary {
  totalCost: number
  directMaterialCost: number
  directLaborCost: number
  manufacturingOverhead: number
  otherCost: number
  costByCategory?: Record<string, number>
  costByProduct?: Record<string, number>
}

export interface CostByBatch {
  batchNo: string
  productName: string
  totalCost: number
  unitCost: number
  quantity: number
}

export interface CostCollectionQueryParams extends QueryParams {
  collectionType?: string
  productId?: number
  batchNo?: string
  status?: string
  periodStart?: string
  periodEnd?: string
}

export function listCollections(params?: CostCollectionQueryParams): Promise<ApiResponse<{ list: CostCollection[]; total: number }>> {
  return request.get('/cost-collections', { params })
}

export function getCollection(id: number): Promise<ApiResponse<CostCollection>> {
  return request.get(`/cost-collections/${id}`)
}

export function createCollection(data: Partial<CostCollection>): Promise<ApiResponse<CostCollection>> {
  return request.post('/cost-collections', data)
}

export function updateCollection(id: number, data: Partial<CostCollection>): Promise<ApiResponse<CostCollection>> {
  return request.put(`/cost-collections/${id}`, data)
}

export function deleteCollection(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/cost-collections/${id}`)
}

export function getAnalysisSummary(params?: { periodStart?: string; periodEnd?: string }): Promise<ApiResponse<CostAnalysisSummary>> {
  return request.get('/cost-collections/analysis/summary', { params })
}

export function getCostByBatch(params?: { periodStart?: string; periodEnd?: string; productId?: number }): Promise<ApiResponse<CostByBatch[]>> {
  return request.get('/cost-collections/analysis/by-batch', { params })
}
