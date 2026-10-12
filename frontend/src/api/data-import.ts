import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface ImportTemplate {
  id: number;
  template_code: string;
  template_name: string;
  description: string;
  module: 'customer' | 'supplier' | 'product' | 'inventory' | 'sales' | 'purchase' | 'finance';
  file_format: 'xlsx' | 'csv' | 'json';
  columns: ImportColumn[];
  // P2-9c 修复（批次 82 v1 复审）：样本数据结构不固定，any[] → unknown[]
  sample_data: unknown[];
  status: 'active' | 'inactive';
  created_at: string;
  updated_at: string;
}

export interface ImportColumn {
  key: string;
  label: string;
  type: 'string' | 'number' | 'date' | 'boolean';
  required: boolean;
  // 批次 98 P2-D 修复（v5 复审）：原 any 改为联合类型，与 type 字段一致
  default_value?: string | number | boolean | null;
  validation_rule?: string;
}

export interface ImportTask {
  id: number;
  task_code: string;
  template_id: number;
  template_name: string;
  file_name: string;
  file_path: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  total_rows: number;
  processed_rows: number;
  success_rows: number;
  failed_rows: number;
  error_log: string;
  created_by: number;
  created_by_name: string;
  created_at: string;
  completed_at: string;
}

// 后端 import_export_handler::list_import_templates 只有 State 提取器，没有 Query<T>：
// 原先声明的 params? 是撒谎签名（所有调用点也不传），删掉。
export function getImportTemplateList(): Promise<ApiResponse<ImportTemplate[]>> {
  return request.get('/data-import/templates');
}

export function getImportTemplate(id: number): Promise<ApiResponse<ImportTemplate>> {
  return request.get(`/data-import/templates/${id}`);
}

// 规则 0（禁止指向不存在端点的封装）：后端 routes/mod.rs data_import_routes
// 对 /data-import/templates 仅注册 GET（含 /{id} 的 GET/PUT/DELETE/download），
// 全库无创建 handler —— 原 createImportTemplate 指向不存在的 POST，调用必 405，
// 封装已删除。"新建导入模板"能力属后端缺口（待补 CreateImportTemplateRequest + 路由），
// 已登记串行清单，前端不得为其伪造契约。

/**
 * PUT /data-import/templates/{id} 载荷（唯一真相：import_export_handler::UpdateImportTemplateRequest）。
 * 全字段 Option（逐键 if let Some 更新）；template_name 后端 #[validate(length(min=1,max=100))]
 * → 空串必须省略该键（发送 "" 直接 422）；columns 元素键见 ImportColumn（后端 ImportColumnDto）。
 * DTO 无 template_code/id/created_at/updated_at/import_type：模板编码创建后不可改
 * （可改性属产品决策，已登记串行清单）。
 */
export interface UpdateImportTemplateRequest {
  template_name?: string;
  description?: string;
  module?: string;
  file_format?: string;
  columns?: ImportColumn[];
  sample_data?: unknown[];
  status?: string;
}

export function updateImportTemplate(
  id: number,
  data: UpdateImportTemplateRequest
): Promise<ApiResponse<ImportTemplate>> {
  return request.put(`/data-import/templates/${id}`, data);
}

export function deleteImportTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/data-import/templates/${id}`);
}

export function getImportTaskList(): Promise<ApiResponse<ImportTask[]>> {
  return request.get('/data-import/tasks');
}

export function getImportTask(id: number): Promise<ApiResponse<ImportTask>> {
  return request.get(`/data-import/tasks/${id}`);
}

export function uploadImportFile(templateId: number, file: File): Promise<ApiResponse<ImportTask>> {
  const formData = new FormData();
  formData.append('file', file);
  formData.append('template_id', templateId.toString());
  return request.post('/data-import/tasks', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  });
}

export function cancelImportTask(id: number): Promise<ApiResponse<void>> {
  return request.post(`/data-import/tasks/${id}/cancel`);
}

export function retryImportTask(id: number): Promise<ApiResponse<void>> {
  return request.post(`/data-import/tasks/${id}/retry`);
}

export function downloadImportTemplate(id: number): Promise<Blob> {
  return request.get(`/data-import/templates/${id}/download`, {
    responseType: 'blob',
  });
}

export function downloadErrorLog(taskId: number): Promise<Blob> {
  return request.get(`/data-import/tasks/${taskId}/error-log`, {
    responseType: 'blob',
  });
}
