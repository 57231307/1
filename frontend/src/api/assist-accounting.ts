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

// P2-9c 修复（批次 82 v1 复审）：辅助核算维度列表查询参数强类型化
export interface AssistDimensionQueryParams {
  page?: number
  page_size?: number
  dimension_code?: string
}

// P2-9c 修复（批次 82 v1 复审）：辅助核算记录列表查询参数强类型化
export interface AssistRecordQueryParams {
  page?: number
  page_size?: number
  dimension_code?: string
  business_type?: string
  accounting_period?: string
  warehouse_id?: number
}

// P2-9c 修复（批次 82 v1 复审）：辅助核算汇总查询参数强类型化
export interface AssistSummaryQueryParams {
  dimension_code?: string
  accounting_period?: string
}

export const listAssistDimensions = (params?: AssistDimensionQueryParams) =>
  request.get('/assist-accounting/dimensions', { params })

export const listAssistRecords = (params?: AssistRecordQueryParams) =>
  request.get('/assist-accounting/records', { params })

export const queryAssistRecords = listAssistRecords

export const getAssistRecordsByFiveDimension = (fiveDimensionId: number) =>
  request.get(`/assist-accounting/records/five-dimension/${fiveDimensionId}`)

export const getAssistSummary = (params?: AssistSummaryQueryParams) =>
  request.get('/assist-accounting/summary', { params })
