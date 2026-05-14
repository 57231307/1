import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface PurchaseInspectionRecord {
  id: number
  recordNo: string
  purchaseOrderId: number
  purchaseOrderNo: string
  productId: number
  productName: string
  batchNo: string
  inspectionDate: string
  inspector: string
  result: 'pass' | 'fail' | 'pending'
  sampleQuantity: number
  qualifiedQuantity: number
  remark: string
  createdAt?: string
}

export interface PurchaseInspectionQueryParams extends QueryParams {
  purchaseOrderId?: number
  productId?: number
  batchNo?: string
  result?: string
  inspectionDateStart?: string
  inspectionDateEnd?: string
}

export function listPurchaseInspectionRecords(params?: PurchaseInspectionQueryParams): Promise<ApiResponse<{ list: PurchaseInspectionRecord[]; total: number }>> {
  return request.get('/purchases/inspections', { params })
}

export function getPurchaseInspectionRecord(id: number): Promise<ApiResponse<PurchaseInspectionRecord>> {
  return request.get(`/purchases/inspections/${id}`)
}

export function createPurchaseInspectionRecord(data: Partial<PurchaseInspectionRecord>): Promise<ApiResponse<PurchaseInspectionRecord>> {
  return request.post('/purchases/inspections', data)
}

export function updatePurchaseInspectionRecord(id: number, data: Partial<PurchaseInspectionRecord>): Promise<ApiResponse<PurchaseInspectionRecord>> {
  return request.put(`/purchases/inspections/${id}`, data)
}

export function deletePurchaseInspectionRecord(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchases/inspections/${id}`)
}