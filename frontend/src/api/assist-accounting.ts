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

export const listAssistDimensions = (params?: any) =>
  request.get('/assist-accounting/dimensions', { params })

export const queryAssistRecords = listAssistRecords

export const listAssistRecords = (params?: any) =>
  request.get('/assist-accounting/records', { params })

export const getAssistRecordsByFiveDimension = (fiveDimensionId: number) =>
  request.get(`/assist-accounting/records/five-dimension/${fiveDimensionId}`)

export const getAssistSummary = () =>
  request.get('/assist-accounting/summary')
