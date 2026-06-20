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

export interface TraceChainResponse {
  id?: number
  trace_chain_id: string
  five_dimension_id: number
  business_type: string
  business_id: number
  relation_type: string
  created_at?: string
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any
}

export interface FullTraceChainResponse {
  trace_chain_id: string
  five_dimension_id: number
  traces: TraceChainResponse[]
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any
}

export const getTraceByFiveDimension = (fiveDimensionId: number | string) =>
  request.get(`/business-trace/five-dimension/${fiveDimensionId}`)

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const forwardTrace = (params?: any) => request.get('/business-trace/forward', { params })

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const backwardTrace = (params?: any) => request.get('/business-trace/backward', { params })

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const createTraceSnapshot = (traceChainId: string, data?: any) =>
  request.post(`/business-trace/snapshot/${traceChainId}`, data)
