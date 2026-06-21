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
  [key: string]: any
}

export interface AssistRecordResponse {
  id?: number
  record_no: string
  accounting_period: string
  dimension_code: string
  business_type: string
  amount: number
  [key: string]: any
}

export interface AssistSummaryResponse {
  dimension_code: string
  dimension_name: string
  total_amount: number
  [key: string]: any
}

export const listAssistDimensions = (params?: any) =>
  request.get('/assist-accounting/dimensions', { params })

export const listAssistRecords = (params?: any) =>
  request.get('/assist-accounting/records', { params })

export const queryAssistRecords = listAssistRecords

export const getAssistRecordsByFiveDimension = (fiveDimensionId: number) =>
  request.get(`/assist-accounting/records/five-dimension/${fiveDimensionId}`)

export const getAssistSummary = (params?: any) =>
  request.get('/assist-accounting/summary', { params })
