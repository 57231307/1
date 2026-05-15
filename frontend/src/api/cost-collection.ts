import request from './request'

export interface CostCollection {
  id?: number
  collection_no: string
  collection_date: string
  cost_object_type?: string
  cost_object_id?: number
  cost_object_no?: string
  batch_no?: string
  color_no?: string
  workshop?: string
  direct_material: number
  direct_labor: number
  manufacturing_overhead: number
  processing_fee: number
  dyeing_fee: number
  output_quantity_meters?: number
  output_quantity_kg?: number
  total_cost?: number
  unit_cost_meters?: number
  unit_cost_kg?: number
  status: string
  auditor_id?: number
  audit_comment?: string
  audit_time?: string
  created_at?: string
  updated_at?: string
}

export interface QueryParams {
  batch_no?: string
  color_no?: string
  page?: number
  page_size?: number
}

export const listCollections = (params?: QueryParams) =>
  request.get('/cost-collections', { params })

export const getCollection = (id: number) =>
  request.get(`/cost-collections/${id}`)

export const createCollection = (data: Partial<CostCollection>) =>
  request.post('/cost-collections', data)

export const updateCollection = (id: number, data: Partial<CostCollection>) =>
  request.put(`/cost-collections/${id}`, data)

export const deleteCollection = (id: number) =>
  request.delete(`/cost-collections/${id}`)

export const auditCollection = (id: number, approved: boolean, comment?: string) =>
  request.post(`/cost-collections/${id}/audit`, { approved, comment })

export const getCostAnalysisSummary = (params?: { start_date?: string; end_date?: string }) =>
  request.get('/cost-collections/analysis/summary', { params })

export const getCostByBatch = (batch_no?: string) =>
  request.get('/cost-collections/analysis/by-batch', { params: { batch_no } })
