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

export const listAssistRecords = (params?: any) =>
  request.get('/assist-accounting/records', { params })

export const getAssistRecordsByFiveDimension = (fiveDimensionId: number) =>
  request.get(`/assist-accounting/records/five-dimension/${fiveDimensionId}`)

export const getAssistSummary = () =>
  request.get('/assist-accounting/summary')
