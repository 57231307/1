import { request } from './request';
import type { ApiResponse, PageResult } from '@/types/api';

export interface ReportField {
  key: string;
  label: string;
  type: 'string' | 'number' | 'date' | 'boolean';
  sortable: boolean;
}

export interface ReportFilterCondition {
  field: string;
  operator: 'eq' | 'ne' | 'gt' | 'lt' | 'gte' | 'lte' | 'contains' | 'in' | 'between';
  // 批次 98 P2-D 修复（v5 复审）：原 any 改为联合类型，覆盖所有 operator 的取值
  value: string | number | boolean | string[] | number[] | null;
}

export interface ReportTemplateField {
  field_key: string;
  display_label: string;
  visible: boolean;
  width?: number;
  format?: string;
}

export interface ReportTemplate {
  id: number;
  name: string;
  description: string;
  type: string;
  category: string;
  fields: ReportTemplateField[];
  filters: ReportFilterCondition[];
  group_by: string[];
  sort_by: { field: string; direction: 'asc' | 'desc' }[];
  chart_type: 'none' | 'bar' | 'line' | 'pie' | 'area';
  is_system: boolean;
  created_at: string;
  updated_at: string;
  created_by?: string;
}

export interface ReportSubscription {
  id: number;
  template_id: number;
  template_name: string;
  schedule: 'daily' | 'weekly' | 'monthly';
  schedule_time: string;
  recipients: string[];
  format: 'pdf' | 'excel' | 'both';
  active: boolean;
  created_at: string;
  last_sent_at?: string;
}

export interface CreateTemplateRequest {
  name: string;
  description?: string;
  type: string;
  category: string;
  fields: ReportTemplateField[];
  filters?: ReportFilterCondition[];
  group_by?: string[];
  sort_by?: { field: string; direction: 'asc' | 'desc' }[];
  chart_type?: string;
}

export interface UpdateTemplateRequest {
  name?: string;
  description?: string;
  fields?: ReportTemplateField[];
  filters?: ReportFilterCondition[];
  group_by?: string[];
  sort_by?: { field: string; direction: 'asc' | 'desc' }[];
  chart_type?: string;
}

export interface CreateSubscriptionRequest {
  template_id: number;
  schedule: 'daily' | 'weekly' | 'monthly';
  schedule_time: string;
  recipients: string[];
  format: 'pdf' | 'excel' | 'both';
}

export interface UpdateSubscriptionRequest {
  schedule?: 'daily' | 'weekly' | 'monthly';
  schedule_time?: string;
  recipients?: string[];
  format?: 'pdf' | 'excel' | 'both';
  active?: boolean;
}

// 后端 report_enhanced_handler::get_available_fields 返回 ApiResponse<serde_json::Value>，
// data 由 json!({ "template_type": ..., "fields": fields }) 构造，真实列表键为 fields。
export function getAvailableFields(
  templateType: string
): Promise<ApiResponse<{ template_type: string; fields: ReportField[] }>> {
  return request.get(`/reports/enhanced/fields/${templateType}`);
}

export function exportReport(
  templateId: number,
  params: {
    format: 'pdf' | 'excel';
    date_range?: { start: string; end: string };
    filters?: ReportFilterCondition[];
  }
): Promise<Blob> {
  return request.post(`/reports/enhanced/templates/${templateId}/export`, params, {
    responseType: 'blob',
  });
}

// P2-16 修复（批次 86 v2 复审）：previewReport ApiResponse<any> → 显式接口
// 增强预览契约修复：键名/形状逐字对齐后端 json! 构造（report_enhanced_handler.rs:512-518）
// 与其数据源 execute_custom_report 的返回类型 (Vec<String>, Vec<Vec<String>>, u64)
// （report_template_service.rs:559）——columns 为字符串表头数组，data 为「行=字符串数组」的二维数组，
// 而非此前臆造的 fields/rows 对象形状。

/** 报表预览结果 */
export interface ReportPreviewResult {
  template_id: number;
  columns: string[];
  data: string[][];
  total: number;
  preview_rows: number;
}

// 后端 preview_template 无 Query 提取器，page/page_size 会被静默丢弃，故不接收 params。
export function previewReport(templateId: number): Promise<ApiResponse<ReportPreviewResult>> {
  return request.get(`/reports/enhanced/templates/${templateId}/preview`);
}

export function getSubscriptionList(
  params?: Record<string, unknown>
): Promise<ApiResponse<PageResult<ReportSubscription>>> {
  return request.get('/reports/enhanced/subscriptions', { params });
}

export function createSubscription(
  data: CreateSubscriptionRequest
): Promise<ApiResponse<ReportSubscription>> {
  return request.post('/reports/enhanced/subscriptions', data);
}

export function updateSubscription(
  id: number,
  data: UpdateSubscriptionRequest
): Promise<ApiResponse<ReportSubscription>> {
  return request.put(`/reports/enhanced/subscriptions/${id}`, data);
}

export function deleteSubscription(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/reports/enhanced/subscriptions/${id}`);
}

export function toggleSubscription(id: number): Promise<ApiResponse<ReportSubscription>> {
  return request.post(`/reports/enhanced/subscriptions/${id}/toggle`);
}

export function sendSubscriptionNow(id: number): Promise<ApiResponse<{ message: string }>> {
  return request.post(`/reports/enhanced/subscriptions/${id}/send`);
}

// ===== 报表模板 CRUD：统一出口（重复实现收敛自 report-templates.ts）=====
export {
  getReportTemplateList,
  getReportTemplate,
  createReportTemplate,
  updateReportTemplate,
  deleteReportTemplate,
} from './report-templates';
