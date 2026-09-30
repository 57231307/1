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

// 质检记录更新载荷 —— 对齐后端 UpdateInspectionRecordRequest（handlers/quality_inspection_handler.rs，
// 三态语义 RFC 7386）：键缺席=保持原值、显式 null=清空为 NULL（仅下列可空列）、有值=覆盖。
// 可空列依据 m0005/system 域 DDL（batch_no m0005:162、inspector_id :167、qualified_qty :169、
// unqualified_qty :170 及 system/mod.rs:183-195 补列 color_no/defect_type/dye_lot_no/grade/
// qualification_rate/remark）。
// NOT NULL/模型非 Option 列（inspection_type/inspection_date/total_qty/inspected_qty/
// inspection_result）不声明 null——显式 null 会被后端 business_displayable 拒绝。
export interface UpdateQualityRecordPayload {
  inspection_type?: string;
  batch_no?: string | null;
  inspection_date?: string;
  inspector_id?: number | null;
  total_qty?: string | number;
  inspected_qty?: string | number;
  qualified_qty?: string | number | null;
  unqualified_qty?: string | number | null;
  qualification_rate?: string | number | null;
  inspection_result?: string;
  remark?: string | null;
  defect_type?: string | null;
  grade?: string | null;
  color_no?: string | null;
  dye_lot_no?: string | null;
}

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

/**
 * 更新质量标准载荷：对齐后端 UpdateQualityStandardRequest
 * （handlers/quality_standard_handler.rs:57-68，字段 standard_name/standard_type/content/status/remark 全 Option）。
 * 注意后端字段名是 standard_type，实体出参 QualityStandard 里才是 type；
 * id / standard_code / version / attachments 不在更新契约内，提交会被 serde 静默丢弃，禁止放入载荷。
 * 状态流转（审批/驳回/发布/归档）走各自端点，更新接口不提交 status。
 */
export interface UpdateQualityStandardPayload {
  standard_name?: string;
  standard_type?: string;
  content?: string;
  remark?: string;
}

export function updateQualityStandard(
  id: number,
  data: UpdateQualityStandardPayload
): Promise<ApiResponse<QualityStandard>> {
  return request.put(`/quality-standards/${id}`, data);
}

export function deleteQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/quality-standards/${id}`);
}

export function approveQualityStandard(
  id: number,
  data?: { approval_comment?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/approve`, data || {});
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

/**
 * 不合格品处理方式取值：与后端 services/quality_inspection_service.rs:45-47 常量逐字一致
 * （downgrade_sale 降级销售 / rework 返工 / scrap 报废）。
 * 后端会按质检记录等级校验合法组合（A 级拒绝处理，B 级必须降级销售，C 级必须返工或报废，
 * 见 validate_handling_method_by_grade，quality_inspection_service.rs:73-），非法组合返回业务错误。
 */
export type DefectHandlingMethod = 'downgrade_sale' | 'rework' | 'scrap';

/**
 * 处理缺陷请求体：对齐后端 ProcessUnqualifiedRequest
 * （services/quality_inspection_service.rs:179-186）。
 * unqualified_qty / unqualified_reason / handling_method 为非 Option 必填，缺任一项即被 serde 反序列化拒绝（422）；
 * remark / handling_result 为 Option<String>，空值时必须省略该键。
 * unqualified_qty 为 rust_decimal，提交 number 或十进制字符串均可。
 */
export interface ProcessDefectPayload {
  unqualified_qty: string | number;
  unqualified_reason: string;
  handling_method: DefectHandlingMethod;
  remark?: string;
  handling_result?: string;
}

/**
 * 处理缺陷出参：后端 handlers/quality_inspection_handler.rs::process_defect 返回
 * models/unqualified_product.rs::Model；Decimal 序列化为字符串，渲染前先 Number() 归一。
 */
export interface UnqualifiedProductRecord {
  id: number;
  unqualified_no: string;
  inspection_id: number | null;
  product_id: number;
  batch_no: string | null;
  unqualified_qty: string;
  unqualified_reason: string;
  handling_method: string;
  handling_status: string;
  handling_by: number | null;
  handling_at: string | null;
  remark: string | null;
  grade: string | null;
  handling_result: string | null;
  created_at: string;
  updated_at: string;
  stock_grade_synced: boolean;
  stock_id: number | null;
  scrap_approval_status: string;
  approver_id_fin: number | null;
  approver_id_gm: number | null;
  approved_at_fin: string | null;
  approved_at_gm: string | null;
  scrap_loss_amount: string | null;
}

export function processDefect(
  id: number,
  data: ProcessDefectPayload
): Promise<ApiResponse<UnqualifiedProductRecord>> {
  return request.post(`/production/quality-inspection/defects/${id}/process`, data);
}

export function archiveQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/archive`);
}
