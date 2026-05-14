import { request } from './request'
import type { ApiResponse } from './request'

export interface BusinessTrace {
  id: number
  five_dimension_id: string
  node_type: string
  node_id: string
  node_name: string
  operation_time: string
  operator: string
  description?: string
}

export interface TraceNode {
  id: number
  type: string
  name: string
  status: string
  data: any
}

export interface TraceChain {
  id: number
  five_dimension_id: string
  nodes: TraceNode[]
  links: any[]
}

export interface ForwardTraceParams {
  five_dimension_id: string
  max_depth?: number
}

export interface BackwardTraceParams {
  five_dimension_id: string
  max_depth?: number
}

export function getTraceByFiveDimension(fiveDimensionId: string): Promise<ApiResponse<BusinessTrace[]>> {
  return request.get(`/business-trace/${encodeURIComponent(fiveDimensionId)}`)
}

export function forwardTrace(params: ForwardTraceParams): Promise<ApiResponse<TraceChain>> {
  return request.get('/business-trace/forward', { params })
}

export function backwardTrace(params: BackwardTraceParams): Promise<ApiResponse<TraceChain>> {
  return request.get('/business-trace/backward', { params })
}

export function createTraceSnapshot(traceChainId: number): Promise<ApiResponse<TraceChain>> {
  return request.post(`/business-trace/${traceChainId}/snapshot`)
}

export function getTraceChain(id: number): Promise<ApiResponse<TraceChain>> {
  return request.get(`/business-trace/chain/${id}`)
}