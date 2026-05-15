import { request } from './request'
import type { ApiResponse } from './request'

export interface TraceNode {
  id: number
  businessType: string
  businessId: number
  action: string
  timestamp: string
  operator?: string
  data?: Record<string, any>
  children?: TraceNode[]
}

export interface TraceChain {
  id: number
  chainId: string
  rootType: string
  rootId: number
  nodes: TraceNode[]
  createdAt: string
  updatedAt: string
}

export interface TraceSnapshot {
  id?: number
  traceChainId?: number
  snapshotData: Record<string, any>
  snapshotType: string
  version?: number
  createdAt?: string
}

export interface BusinessTraceQueryParams {
  fiveDimensionId?: number
  businessType?: string
  businessId?: number
  startDate?: string
  endDate?: string
}

export const businessTraceApi = {
  getByFiveDimension: (fiveDimensionId: number) =>
    request.get<ApiResponse<TraceChain>>(`/business-trace/five-dimension/${fiveDimensionId}`),

  forwardTrace: (businessType: string, businessId: number) =>
    request.get<ApiResponse<{ trace: TraceNode[] }>>(`/business-trace/forward`, {
      params: { businessType, businessId }
    }),

  backwardTrace: (businessType: string, businessId: number) =>
    request.get<ApiResponse<{ trace: TraceNode[] }>>(`/business-trace/backward`, {
      params: { businessType, businessId }
    }),

  createSnapshot: (traceChainId: number, data: Partial<TraceSnapshot>) =>
    request.post<ApiResponse<TraceSnapshot>>(`/business-trace/snapshot/${traceChainId}`, data),
}
