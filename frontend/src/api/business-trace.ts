import { request } from './request'

export interface TraceRecord {
  id?: number
  trace_chain_id: string
  five_dimension_id: number
  business_type: string
  business_id: number
  relation_type: string
  created_at?: string
}

// P3 维度 9 修复（批次 87）：索引签名 any → unknown
export interface TraceChainResponse {
  id?: number
  trace_chain_id: string
  five_dimension_id: number
  business_type: string
  business_id: number
  relation_type: string
  created_at?: string
  [key: string]: unknown
}

export interface FullTraceChainResponse {
  trace_chain_id: string
  five_dimension_id: number
  traces: TraceChainResponse[]
  [key: string]: unknown
}

// P2-9c 修复（批次 82 v1 复审）：业务追溯查询参数强类型化
export interface TraceQueryParams {
  trace_chain_id?: string
  business_type?: string
  business_id?: number
  five_dimension_id?: number
  supplier_id?: number
  customer_id?: number
  batch_no?: string
}

export const getTraceByFiveDimension = (fiveDimensionId: number | string) =>
  request.get(`/business-trace/five-dimension/${fiveDimensionId}`)

export const forwardTrace = (params?: TraceQueryParams) =>
  request.get('/business-trace/forward', { params })

export const backwardTrace = (params?: TraceQueryParams) =>
  request.get('/business-trace/backward', { params })

// P2-9c 修复（批次 82 v1 复审）：创建追溯快照请求 DTO
export interface TraceSnapshotCreateDto {
  snapshot_type?: string
  remark?: string
  metadata?: unknown
}

export const createTraceSnapshot = (traceChainId: string, data?: TraceSnapshotCreateDto) =>
  request.post(`/business-trace/snapshot/${traceChainId}`, data)
