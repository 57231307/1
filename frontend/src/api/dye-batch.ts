import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface DyeBatch {
  id: number;
  batch_no: string;
  color_code: string;
  color_name: string;
  greige_fabric_id: number;
  greige_fabric_name: string;
  planned_quantity: number;
  actual_quantity: number;
  unit: string;
  recipe_id: number;
  recipe_name: string;
  status:
    | 'pending_schedule'
    | 'scheduled'
    | 'preparing'
    | 'dyeing'
    | 'washing'
    | 'fixing'
    | 'dehydrating'
    | 'drying'
    | 'inspecting'
    | 'stored'
    | 'shipped'
    | 'cancelled'
    | 'terminated'
    | 'rework'
    | 'on_hold'
    | 'failed';
  start_date: string;
  end_date: string;
  machine_code: string;
  operator: string;
  remark: string;
  created_by: number;
  created_by_name: string;
  created_at: string;
  updated_at: string;
}

/**
 * 缸号（染色批次）列表查询参数——严格对齐后端 dye_batch_handler.rs::DyeBatchListQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * 支持真实筛选：色号 color_no（空即白坯）、缸号 dye_lot_no、批次 batch_no、状态 status。
 */
export interface DyeBatchListParams {
  page?: number;
  page_size?: number;
  color_no?: string;
  dye_lot_no?: string;
  batch_no?: string;
  status?: string;
}

export function getDyeBatchList(
  params?: DyeBatchListParams
): Promise<ApiResponse<{ items: DyeBatch[]; total: number; page: number; page_size: number }>> {
  return request.get('/production/dye-batches', { params });
}

export function getDyeBatch(id: number): Promise<ApiResponse<DyeBatch>> {
  return request.get(`/production/dye-batches/${id}`);
}

export function createDyeBatch(data: Partial<DyeBatch>): Promise<ApiResponse<DyeBatch>> {
  return request.post('/production/dye-batches', data);
}

export function updateDyeBatch(
  id: number,
  data: Partial<DyeBatch>
): Promise<ApiResponse<DyeBatch>> {
  return request.put(`/production/dye-batches/${id}`, data);
}

export function deleteDyeBatch(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/dye-batches/${id}`);
}

export function completeDyeBatch(id: number): Promise<ApiResponse<void>> {
  return request.post(`/production/dye-batches/${id}/complete`);
}

export function getDyeBatchesByColor(colorCode: string): Promise<ApiResponse<DyeBatch[]>> {
  return request.get(`/production/dye-batches/by-color/${colorCode}`);
}

export function exportDyeBatches(params?: DyeBatchListParams): Promise<Blob> {
  return request.get('/production/dye-batches/export', { params, responseType: 'blob' });
}
