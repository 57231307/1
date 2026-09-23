import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 缸号列表/详情出参。列表 handler 直接 `to_value(dye_batch::Model)`
 * （handlers/dye_batch_handler.rs::list_dye_batches），故键 = `backend/src/models/dye_batch.rs`
 * 的字段名，可空列（Option）在此标 `| null`，NOT NULL 列不得标可选（否则掩盖缺键）。
 */
export interface DyeBatch {
  id: number;
  batch_no: string;
  greige_fabric_id: number | null;
  color_code: string;
  color_name: string;
  color_no: string | null;
  dye_lot_no: string;
  planned_quantity: number | null;
  status: string | null;
  started_at: string | null;
  completed_at: string | null;
  is_deleted: boolean | null;
  created_at: string;
  updated_at: string;
  // 名称列：models/dye_batch.rs 的 Model 只有 greige_fabric_id、无名称列，
  // 需后端按 §5 范式 LEFT JOIN greige_fabric 取 fabric_name 输出为 greige_fabric_name。
  greige_fabric_name: string | null;
  // 备注列：models/dye_batch.rs 的 Model 无 remarks 列，需后端补 dye_batch.remarks。
  remarks: string | null;
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
