import { request } from './request';
import type { ApiResponse } from '@/types/api';
import type { PrintTaskJobRef } from './print-tasks';

export interface PrintTemplate {
  id: number;
  template_code: string;
  template_name: string;
  description: string;
  module: 'sales' | 'purchase' | 'inventory' | 'finance' | 'production' | 'logistics';
  type: 'order' | 'invoice' | 'receipt' | 'label' | 'report' | 'custom';
  /** 渲染链路查找键：打印任务按本列取「启用且默认」模板，取值受后端 print_service 支持集约束 */
  doc_type: string;
  paper_size: 'A4' | 'A5' | 'B5' | 'Letter' | 'Custom';
  orientation: 'portrait' | 'landscape';
  /** 缺省打印份数（print_tasks 未覆盖份数时取本列） */
  copies_default: number;
  /** 缺省打印目标符号名（print_targets.target_code）；为空时发起打印必须显式指定目标 */
  default_target_name: string | null;
  content: string;
  css_styles: string;
  variables: Record<string, unknown>;
  status: 'active' | 'inactive';
  is_default: boolean;
  created_by: number;
  created_by_name: string;
  created_at: string;
  updated_at: string;
}

// 后端 print_handler::list_print_templates 无 Query<T> 提取器，传了也会被忽略 -> 不再声明参数
export function getPrintTemplateList(): Promise<ApiResponse<PrintTemplate[]>> {
  return request.get('/print-templates');
}

export function getPrintTemplate(id: number): Promise<ApiResponse<PrintTemplate>> {
  return request.get(`/print-templates/${id}`);
}

/**
 * POST /print-templates 载荷（唯一真相：print_handler::CreatePrintTemplateRequest）。
 * template_name 必填且后端 #[validate(length(min=1,max=100))] → 空串 422；
 * doc_type 必填且须属后端 print_service 支持的单据类型集，否则 400 点名非法值——
 * 缺查找键的模板永不会被渲染链路命中，故不允许省略；
 * type 键经 #[serde(rename="type")] 对应 Rust template_type；
 * DTO 无 id/created_by/created_by_name/created_at/updated_at（禁止整实体 Partial 当载荷）。
 */
export interface CreatePrintTemplateRequest {
  template_name: string;
  doc_type: string;
  template_code?: string;
  description?: string;
  module?: PrintTemplate['module'];
  type?: PrintTemplate['type'];
  paper_size?: PrintTemplate['paper_size'];
  orientation?: PrintTemplate['orientation'];
  copies_default?: number;
  default_target_name?: string;
  content?: string;
  css_styles?: string;
  variables?: Record<string, unknown>;
  is_default?: boolean;
  status?: PrintTemplate['status'];
}

/**
 * PUT /print-templates/{id} 载荷（唯一真相：UpdatePrintTemplateRequest，全字段 Option）。
 * template_name 可选但带 length(min=1) 校验 → 空串必须省略该键，不得发送 ""。
 * default_target_name 为可空列三态：键缺席=保持原值、显式 null=清空、有值=覆盖。
 */
export interface UpdatePrintTemplateRequest {
  template_name?: string;
  doc_type?: string;
  template_code?: string;
  description?: string;
  module?: PrintTemplate['module'];
  type?: PrintTemplate['type'];
  paper_size?: PrintTemplate['paper_size'];
  orientation?: PrintTemplate['orientation'];
  copies_default?: number;
  default_target_name?: string | null;
  content?: string;
  css_styles?: string;
  variables?: Record<string, unknown>;
  is_default?: boolean;
  status?: PrintTemplate['status'];
}

export function createPrintTemplate(
  data: CreatePrintTemplateRequest
): Promise<ApiResponse<PrintTemplate>> {
  return request.post('/print-templates', data);
}

export function updatePrintTemplate(
  id: number,
  data: UpdatePrintTemplateRequest
): Promise<ApiResponse<PrintTemplate>> {
  return request.put(`/print-templates/${id}`, data);
}

export function deletePrintTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/print-templates/${id}`);
}

// P2-16 修复（批次 86 v2 复审）：previewPrintTemplate ApiResponse<any> → 显式接口

/** 打印模板预览结果 */
export interface PrintTemplatePreviewResult {
  html: string;
  variables: Record<string, unknown>;
  [key: string]: unknown;
}

export function previewPrintTemplate(
  id: number,
  data?: Record<string, unknown>
): Promise<ApiResponse<PrintTemplatePreviewResult>> {
  return request.post(`/print-templates/${id}/preview`, data);
}

/**
 * 以模板发起真实打印的载荷（唯一真相：print_handler::extract_doc_id / extract_target_override）。
 * doc_id 必填：本端点只打真实单据实例，缺 doc_id 后端 fail-closed 400（禁止伪造样例打印），
 * 故类型层把 doc_id 设为必填，杜绝"模板管理页无单据上下文直接点打印"的死路。
 * 打印入口在业务单据侧（由源单带出 doc_id），见 api/print-tasks.ts 与组件 PrintDocumentDialog。
 */
export interface PrintTemplatePrintRequest {
  doc_id: number;
  target_name?: string;
}

/** 以模板发起一次真实打印任务，返回任务引用 { id, job_code } 供轮询 getPrintTask */
export function printTemplate(
  id: number,
  data: PrintTemplatePrintRequest
): Promise<ApiResponse<PrintTaskJobRef>> {
  return request.post(`/print-templates/${id}/print`, data);
}

export function setDefaultPrintTemplate(id: number): Promise<ApiResponse<void>> {
  return request.put(`/print-templates/${id}/set-default`);
}

export function copyPrintTemplate(id: number): Promise<ApiResponse<PrintTemplate>> {
  return request.post(`/print-templates/${id}/copy`);
}
