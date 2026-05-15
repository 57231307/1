import { request } from './request'

export interface CostCollection {
  id?: number
  collection_no: string
  collection_date: string
  batch_no?: string
  color_no?: string
  direct_material: number
  direct_labor: number
  manufacturing_overhead: number
  total_cost?: number
  status: string
}

export const listCollections = (params?: any) =>
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
