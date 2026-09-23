import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

export interface QualityStandard {
  id: number;
  standard_code: string;
  standard_name: string;
  version: string;
  type: 'product' | 'process';
  status: 'draft' | 'approved' | 'published';
  content: string;
  attachments: string[];
  created_by: number;
  created_by_name: string;
  approved_by: number;
  approved_by_name: string;
  approved_at: string;
  created_at: string;
  updated_at: string;
}

/**
 * 质检记录出参：与后端 models/quality_inspection_record.rs 的 Model 字段一一对应。
 * 后端 Decimal 一律序列化为字符串，数量/比率按 string 取用（渲染前转数字）。
 */
export interface QualityRecord {
  id: number;
  inspection_no: string;
  inspection_type: string;
  related_type: string | null;
  related_id: number | null;
  product_id: number;
  batch_no: string | null;
  supplier_id: number | null;
  customer_id: number | null;
  inspection_date: string;
  inspector_id: number | null;
  total_qty: string;
  inspected_qty: string;
  qualified_qty: string | null;
  unqualified_qty: string | null;
  qualification_rate: string | null;
  inspection_result: string;
  remark: string | null;
  defect_type: string | null;
  grade: string | null;
  color_no: string | null;
  dye_lot_no: string | null;
  dye_type: string | null;
  auxiliary_type: string | null;
  temperature: string | null;
  fabric_source: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * 新建质检记录请求体：后端 CreateInspectionRecordRequest 里非 Option 的字段都必须提交
 * （inspection_no / inspection_type / product_id / inspection_date / total_qty /
 * inspected_qty / inspection_result），少一个即被反序列化拒绝。
 */
export interface CreateQualityRecordPayload {
  inspection_no: string;
  inspection_type: string;
  product_id: number;
  inspection_date: string;
  total_qty: string | number;
  inspected_qty: string | number;
  inspection_result: string;
  batch_no?: string;
  inspector_id?: number;
  supplier_id?: number;
  customer_id?: number;
  remark?: string;
  defect_type?: string;
  color_no?: string;
  dye_lot_no?: string;
}

/** 更新质检记录：后端按 Option 逐字段判空更新，只提交改动过的项 */
export type UpdateQualityRecordPayload = Partial<CreateQualityRecordPayload>;

export interface Defect {
  id: number;
  record_id: number;
  defect_type: string;
  defect_description: string;
  severity: 'minor' | 'major' | 'critical';
  quantity: number;
  processed: boolean;
  processed_by: string;
  processed_at: string;
  remark: string;
}

/**
 * 质量标准列表查询参数——严格对齐后端 quality_standard_handler.rs::QualityStandardQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 */
export interface QualityStandardListParams {
  standard_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

export function getQualityStandardList(
  params?: QualityStandardListParams
): Promise<ApiResponse<QualityStandard[]>> {
  // 批次 157d-2 修复：后端 quality-standards 已从 /production 域提升到根级
  return request.get('/quality-standards', { params });
}

export function getQualityStandard(id: number): Promise<ApiResponse<QualityStandard>> {
  return request.get(`/quality-standards/${id}`);
}

export function createQualityStandard(
  data: Partial<QualityStandard>
): Promise<ApiResponse<QualityStandard>> {
  return request.post('/quality-standards', data);
}

export function updateQualityStandard(
  id: number,
  data: Partial<QualityStandard>
): Promise<ApiResponse<QualityStandard>> {
  return request.put(`/quality-standards/${id}`, data);
}

export function deleteQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/quality-standards/${id}`);
}

export function approveQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/approve`);
}

// 批次 157d-2 新增：驳回质量标准
export function rejectQualityStandard(
  id: number,
  data?: { reject_reason?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/reject`, data || {});
}

export function publishQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/publish`);
}

export function getQualityStandardVersions(id: number): Promise<ApiResponse<QualityStandard[]>> {
  return request.get(`/quality-standards/${id}/versions`);
}

// 后端 quality_inspection_handler::list_records 以 success_paginated 返回 PaginatedResponse（items + total + page + page_size）
/**
 * 质检记录列表查询参数——严格对齐后端 quality_inspection_handler.rs::RecordQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * batch_no 为批次号（与 sales 侧契约同名）。
 */
export interface QualityRecordListParams {
  product_id?: number;
  batch_no?: string;
  inspection_type?: string;
  inspection_result?: string;
  page?: number;
  page_size?: number;
}

export function getQualityRecordList(
  params?: QualityRecordListParams
): Promise<ApiResponse<PaginatedResponse<QualityRecord>>> {
  return request.get('/production/quality-inspection/records', { params });
}

export function getQualityRecord(id: number): Promise<ApiResponse<QualityRecord>> {
  return request.get(`/production/quality-inspection/records/${id}`);
}

export function createQualityRecord(
  data: CreateQualityRecordPayload
): Promise<ApiResponse<QualityRecord>> {
  return request.post('/production/quality-inspection/records', data);
}

// 批次 94 P2-12 修复：补全质检记录更新接口（原先前端 API 模块缺失，导致 index.vue 更新占位）
export function updateQualityRecord(
  id: number,
  data: UpdateQualityRecordPayload
): Promise<ApiResponse<QualityRecord>> {
  return request.put(`/production/quality-inspection/records/${id}`, data);
}

/**
 * 缺陷列表查询参数——严格对齐后端 quality_inspection_handler.rs::DefectQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 */
export interface DefectListParams {
  record_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

export function getDefectList(params?: DefectListParams): Promise<ApiResponse<Defect[]>> {
  return request.get('/production/quality-inspection/defects', { params });
}

export function processDefect(id: number, data: { remark: string }): Promise<ApiResponse<void>> {
  return request.post(`/production/quality-inspection/defects/${id}/process`, data);
}

export function archiveQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/archive`);
}
