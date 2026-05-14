import { request } from './request'

export interface AssistDimensionResponse {
  id: number
  dimension_code: string
  dimension_name: string
  description?: string
  is_active: boolean
  sort_order: number
}

export interface AssistRecordResponse {
  id: number
  business_type: string
  business_no: string
  business_id: number
  account_subject_id: number
  debit_amount: number
  credit_amount: number
  five_dimension_id: string
  product_id: number
  batch_no: string
  color_no: string
  dye_lot_no?: string
  grade: string
  warehouse_id: number
  quantity_meters: number
  quantity_kg: number
  workshop_id?: number
  customer_id?: number
  supplier_id?: number
  remarks?: string
  created_at: string
}

export interface AssistSummaryResponse {
  id: number
  accounting_period: string
  dimension_code: string
  dimension_value_id: number
  dimension_value_name: string
  account_subject_id: number
  total_debit: number
  total_credit: number
  total_quantity_meters: number
  total_quantity_kg: number
  record_count: number
}

export interface AssistRecordListResponse {
  records: AssistRecordResponse[]
  total: number
  page: number
  page_size: number
}

export interface AssistRecordQueryParams {
  accounting_period?: string
  dimension_code?: string
  business_type?: string
  warehouse_id?: number
  page?: number
  page_size?: number
}

export interface BusinessQueryParams {
  business_type: string
  business_no: string
}

export interface AssistSummaryQueryParams {
  accounting_period: string
  dimension_code?: string
}

export function listAssistDimensions() {
  return request.get('/assist-accounting/dimensions')
}

export function queryAssistRecords(params?: AssistRecordQueryParams) {
  return request.get('/assist-accounting/records', { params })
}

export function getAssistRecordsByBusiness(params: BusinessQueryParams) {
  return request.get('/assist-accounting/records/by-business', { params })
}

export function getAssistRecordsByFiveDimension(fiveDimensionId: string) {
  return request.get(`/assist-accounting/records/five-dimension/${fiveDimensionId}`)
}

export function getAssistSummary(params: AssistSummaryQueryParams) {
  return request.get('/assist-accounting/summary', { params })
}