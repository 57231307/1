import { request } from './request'

export interface AssistRecord {
  id?: number
  record_no: string
  five_dimension_id: number
  business_type: string
  business_id: number
  amount: number
  created_at?: string
}

export interface AssistDimension {
  id: number
  name: string
  type: string
}

export interface AssistDimensionResponse {
  dimension_code: string
  dimension_name: string
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any
}

export interface AssistRecordResponse {
  id?: number
  record_no: string
  accounting_period: string
  dimension_code: string
  business_type: string
  amount: number
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any
}

export interface AssistSummaryResponse {
  dimension_code: string
  dimension_name: string
  total_amount: number
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const listAssistDimensions = (params?: any) =>
  request.get('/assist-accounting/dimensions', { params })

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const listAssistRecords = (params?: any) =>
  request.get('/assist-accounting/records', { params })

export const queryAssistRecords = listAssistRecords

export const getAssistRecordsByFiveDimension = (fiveDimensionId: number) =>
  request.get(`/assist-accounting/records/five-dimension/${fiveDimensionId}`)

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const getAssistSummary = (params?: any) =>
  request.get('/assist-accounting/summary', { params })
