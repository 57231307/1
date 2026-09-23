import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 报表模板实体——出参键名逐字段对齐后端
 * `backend/src/models/report_template.rs`（handler 直接 `serde_json::to_value(Model)`）。
 * 后端所有字段均序列化（Option 出 null），故可空性严格照实体列：NOT NULL 列不标 `?`。
 * 注意：后端列表查询是 `Entity::find()` 直出 Model，未做 JOIN，故不返回 `created_by_name`
 * （该键在前端曾长期存在但后端从不出参，已删除，列为需要后端 JOIN 的已知缺口）。
 */
export interface ReportTemplate {
  id: number;
  name: string;
  code: string;
  report_type: string;
  template_id: string | null;
  category: string | null;
  data_source: string | null;
  /** 列定义（JSON 数组），NOT NULL；实体类型 sea_orm Json → serde_json::Value */
  columns: unknown;
  /** 以下三个均为 Option<Json>：后端可空且不限形 */
  filters: unknown;
  parameters: Record<string, unknown> | null;
  supported_formats: unknown;
  sort_by: string | null;
  sort_order: string | null;
  data_source_sql: string | null;
  description: string | null;
  is_public: boolean;
  /** 状态词表写入方为 ACTIVE / INACTIVE（report_template_service.rs create/update/delete） */
  status: 'ACTIVE' | 'INACTIVE';
  version: number;
  required_permission: string | null;
  refresh_strategy: string | null;
  cache_ttl_seconds: number | null;
  created_by: number;
  created_at: string;
  updated_at: string;
}

/** GET /reports/enhanced/templates 的列表载荷（handler json!({"items","total"})） */
export interface ReportTemplateListResult {
  items: ReportTemplate[];
  total: number;
}

/** 列表查询参数——对齐后端 `ReportTemplateQuery`（全部可选） */
export interface ReportTemplateListQuery {
  report_type?: string;
  status?: string;
  keyword?: string;
  page?: number;
  page_size?: number;
}

/** 新建请求体——对齐后端 `CreateReportTemplateRequest` 的必填/可选集 */
export interface ReportTemplateCreatePayload {
  name: string;
  code: string;
  report_type: string;
  columns: unknown[];
  category?: string;
  parameters?: Record<string, unknown>;
  description?: string;
  is_public?: boolean;
}

/** 更新请求体——对齐后端 `UpdateReportTemplateRequest`（全部可选，且不含 code/parameters/category） */
export interface ReportTemplateUpdatePayload {
  name?: string;
  report_type?: string;
  columns?: unknown[];
  description?: string;
  status?: string;
  is_public?: boolean;
}

/** 导出请求体——对齐后端 `TemplateExportRequest` */
export interface ReportTemplateExportPayload {
  format?: string;
  title?: string;
}

/**
 * 导出结果——`report_enhanced_handler.rs:488-495` 返回的是 JSON 信封，不是二进制流：
 * `content` 是 base64 编码的文件内容，需前端解码后落盘。
 */
export interface ReportTemplateExportResult {
  template_id: number;
  filename: string;
  size: number;
  content_type: string;
  content: string;
  message: string;
}

/**
 * 预览结果——`report_enhanced_handler.rs:512-518` json! 顶层键：
 * `columns` 为表头名数组（Vec<String>），`data` 为行数组（每行 Vec<String>）。
 */
export interface ReportTemplatePreviewResult {
  template_id: number;
  columns: string[];
  data: string[][];
  total: number;
  preview_rows: number;
}

export function getReportTemplateList(
  query?: ReportTemplateListQuery
): Promise<ApiResponse<ReportTemplateListResult>> {
  return request.get('/reports/enhanced/templates', { params: query });
}

export function getReportTemplate(id: number): Promise<ApiResponse<ReportTemplate>> {
  return request.get(`/reports/enhanced/templates/${id}`);
}

export function createReportTemplate(
  data: ReportTemplateCreatePayload
): Promise<ApiResponse<ReportTemplate>> {
  return request.post('/reports/enhanced/templates', data);
}

export function updateReportTemplate(
  id: number,
  data: ReportTemplateUpdatePayload
): Promise<ApiResponse<ReportTemplate>> {
  return request.put(`/reports/enhanced/templates/${id}`, data);
}

export function deleteReportTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/reports/enhanced/templates/${id}`);
}

/** 预览：后端 preview_template 无 Query<T> 提取器，只按 id 取，不接受查询参数 */
export function previewReportTemplate(
  id: number
): Promise<ApiResponse<ReportTemplatePreviewResult>> {
  return request.get(`/reports/enhanced/templates/${id}/preview`);
}

/** 导出：POST /templates/{id}/export，返回 JSON 信封（base64 content），非二进制流 */
export function exportReportTemplate(
  id: number,
  data: ReportTemplateExportPayload
): Promise<ApiResponse<ReportTemplateExportResult>> {
  return request.post(`/reports/enhanced/templates/${id}/export`, data);
}
