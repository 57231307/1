import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface DyeBatch {
  id: number
  batch_no: string
  color_code: string
  color_name: string
  greige_fabric_id: number
  greige_fabric_name: string
  planned_quantity: number
  actual_quantity: number
  unit: string
  recipe_id: number
  recipe_name: string
  status: 'pending' | 'in_progress' | 'completed' | 'cancelled'
  start_date: string
  end_date: string
  machine_code: string
  operator: string
  remark: string
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export function listDyeBatches(params?: QueryParams): Promise<ApiResponse<DyeBatch[]>> {
  return request.get('/production/dye-batches', { params })
}

export function getDyeBatch(id: number): Promise<ApiResponse<DyeBatch>> {
  return request.get(`/production/dye-batches/${id}`)
}

export function createDyeBatch(data: Partial<DyeBatch>): Promise<ApiResponse<DyeBatch>> {
  return request.post('/production/dye-batches', data)
}

export function updateDyeBatch(
  id: number,
  data: Partial<DyeBatch>
): Promise<ApiResponse<DyeBatch>> {
  return request.put(`/production/dye-batches/${id}`, data)
}

export function deleteDyeBatch(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/dye-batches/${id}`)
}

export function completeDyeBatch(id: number): Promise<ApiResponse<void>> {
  return request.post(`/production/dye-batches/${id}/complete`)
}

export function getDyeBatchesByColor(colorCode: string): Promise<ApiResponse<DyeBatch[]>> {
  return request.get(`/production/dye-batches/by-color/${colorCode}`)
}

export function exportDyeBatches(params?: QueryParams): Promise<Blob> {
  return request.get('/production/dye-batches/export', { params, responseType: 'blob' })
}
