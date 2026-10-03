import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 缸号列表出参：后端 list_dye_batches 返回 DyeBatchDto
 * （handlers/dye_batch_handler.rs:72，SeaORM 列 + greige_fabric_name LEFT JOIN 富化）。
 * rust_decimal 列（planned_quantity）序列化为字符串；展示/计算前 Number() 归一。
 */
export interface DyeBatch {
  id: number;
  batch_no: string;
  greige_fabric_id: number | null;
  color_code: string;
  color_name: string;
  color_no: string | null;
  dye_lot_no: string;
  planned_quantity: string | null;
  /** 完工登记的实际落布重量（kg，Decimal 出参为字符串；未完工为 null） */
  actual_output_kg: string | null;
  /** 完工登记的实际落布长度（米，Decimal 出参为字符串；未完工为 null） */
  actual_output_m: string | null;
  /** 完工登记的坯布投料量（kg，Decimal 出参为字符串；未完工为 null） */
  greige_input_kg: string | null;
  status: string | null;
  started_at: string | null;
  completed_at: string | null;
  is_deleted: boolean | null;
  created_at: string;
  updated_at: string;
  remarks: string | null;
  greige_fabric_name: string | null;
}

/**
 * 创建缸号载荷：逐字段对齐 CreateDyeBatchRequest（dye_batch_handler.rs:42）。
 * batch_no 留空由后端自动生成；status 为 14 态 lifecycle_status 英文 key
 * （省略由后端默认 pending_schedule）。纺织四维口径：染色布必须带缸号 dye_lot_no，
 * color_no 白坯可空但空值必须整键省略（后端把空串当白坯，勿提交 ''）。
 */
export interface CreateDyeBatchPayload {
  batch_no?: string;
  greige_fabric_id?: number;
  color_no?: string;
  dye_lot_no?: string;
  planned_quantity?: number;
  status?: string;
  remarks?: string;
  /** YYYY-MM-DD，落库映射 started_at（dye_batch_handler.rs:177） */
  dye_date?: string;
}

/**
 * 更新缸号载荷：对齐 UpdateDyeBatchRequest（dye_batch_handler.rs:57）。
 * 该 DTO 无 batch_no/dye_date/color_code/remarks 之外的建单键；status 提交即触发
 * 后端状态机流转校验（同态自转不合法），仅在用户显式改状态时携带。
 */
export interface UpdateDyeBatchPayload {
  greige_fabric_id?: number;
  color_no?: string;
  dye_lot_no?: string;
  planned_quantity?: number;
  status?: string;
  remarks?: string;
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

export function createDyeBatch(data: CreateDyeBatchPayload): Promise<ApiResponse<DyeBatch>> {
  return request.post('/production/dye-batches', data);
}

export function updateDyeBatch(
  id: number,
  data: UpdateDyeBatchPayload
): Promise<ApiResponse<DyeBatch>> {
  return request.put(`/production/dye-batches/${id}`, data);
}

export function deleteDyeBatch(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/dye-batches/${id}`);
}

/**
 * 完工登记载荷：逐字段对齐后端 CompleteDyeBatchRequest（dye_batch_handler.rs）。
 * 三值均必填、为正、最多 2 位小数（DECIMAL(12,2)）；数字直接提交（后端 Decimal 反序列化
 * 接受 JSON number），缺失/为 0/负数后端返回 400 且状态不推进。
 */
export interface CompleteDyeBatchPayload {
  actual_output_kg: number;
  actual_output_m: number;
  greige_input_kg: number;
}

export function completeDyeBatch(
  id: number,
  data: CompleteDyeBatchPayload
): Promise<ApiResponse<DyeBatch>> {
  return request.post(`/production/dye-batches/${id}/complete`, data);
}

export function getDyeBatchesByColor(colorCode: string): Promise<ApiResponse<DyeBatch[]>> {
  return request.get(`/production/dye-batches/by-color/${colorCode}`);
}

export function exportDyeBatches(params?: DyeBatchListParams): Promise<Blob> {
  return request.get('/production/dye-batches/export', { params, responseType: 'blob' });
}
