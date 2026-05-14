import { request } from './request'

export interface TraceChainResponse {
  id: number
  trace_chain_id: string
  five_dimension_id: string
  product_id: number
  batch_no: string
  color_no: string
  dye_lot_no?: string
  grade: string
  current_stage: string
  current_bill_type: string
  current_bill_no: string
  quantity_meters: number
  quantity_kg: number
  warehouse_id: number
  supplier_id?: number
  customer_id?: number
  trace_status: string
  created_at: string
}

export interface TraceStageDetail {
  stage_id: number
  stage_name: string
  stage_type: string
  bill_type: string
  bill_no: string
  quantity_meters: number
  quantity_kg: number
  warehouse_id: number
  warehouse_name?: string
  supplier_name?: string
  customer_name?: string
  created_at: string
}

export interface FullTraceChainResponse {
  trace_chain_id: string
  five_dimension_id: string
  product_id: number
  batch_no: string
  color_no: string
  grade: string
  stages: TraceStageDetail[]
  total_stages: number
  start_time: string
  end_time?: string
}

export interface TraceListResponse {
  traces: TraceChainResponse[]
  total: number
}

export interface ForwardTraceParams {
  supplier_id: number
  batch_no: string
}

export interface BackwardTraceParams {
  customer_id: number
  batch_no: string
}

export function getTraceByFiveDimension(fiveDimensionId: string) {
  return request.get(`/api/v1/business-trace/${fiveDimensionId}`)
}

export function forwardTrace(params: ForwardTraceParams) {
  return request.get('/api/v1/business-trace/forward', { params })
}

export function backwardTrace(params: BackwardTraceParams) {
  return request.get('/api/v1/business-trace/backward', { params })
}

export function createTraceSnapshot(traceChainId: string) {
  return request.post(`/api/v1/business-trace/${traceChainId}/snapshot`)
}