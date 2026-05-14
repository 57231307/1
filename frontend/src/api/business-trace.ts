import { request } from './request'
import type { ApiResponse } from './request'

export interface TraceNode {
  id: number
  businessType: string
  businessId: string
  businessNo: string
  businessDate: string
  direction: 'forward' | 'backward'
  quantity: number
  amount?: number
  remark?: string
  createdAt?: string
}

export interface TraceChain {
  fiveDimensionId: string
  nodes: TraceNode[]
  createdAt?: string
}

export interface TraceSnapshot {
  id: number
  traceChainId: number
  fiveDimensionId: string
  snapshotData: any
  createdBy?: string
  createdAt?: string
}

export function getTraceByFiveDimension(fiveDimensionId: string): Promise<ApiResponse<TraceChain>> {
  return request.get(`/business-trace/five-dimension/${encodeURIComponent(fiveDimensionId)}`)
}

export function forwardTrace(params?: { fiveDimensionId?: string; businessId?: string }): Promise<ApiResponse<TraceChain>> {
  return request.get('/business-trace/forward', { params })
}

export function backwardTrace(params?: { fiveDimensionId?: string; businessId?: string }): Promise<ApiResponse<TraceChain>> {
  return request.get('/business-trace/backward', { params })
}

export function createTraceSnapshot(data: { traceChainId?: number; fiveDimensionId?: string; snapshotData?: any }): Promise<ApiResponse<TraceSnapshot>> {
  return request.post('/business-trace/snapshot', data)
}
