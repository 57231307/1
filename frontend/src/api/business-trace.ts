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

export const getTraceByFiveDimension = (fiveDimensionId: number) =>
  request.get(`/business-trace/five-dimension/${fiveDimensionId}`)

export const forwardTrace = (params?: any) =>
  request.get('/business-trace/forward', { params })

export const backwardTrace = (params?: any) =>
  request.get('/business-trace/backward', { params })

export const createTraceSnapshot = (data: any) =>
  request.post('/business-trace/snapshot', data)
