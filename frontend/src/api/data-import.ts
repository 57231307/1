import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface ImportTemplate {
  id: number
  template_code: string
  template_name: string
  description: string
  module: 'customer' | 'supplier' | 'product' | 'inventory' | 'sales' | 'purchase' | 'finance'
  file_format: 'xlsx' | 'csv' | 'json'
  columns: ImportColumn[]
  // P2-9c 修复（批次 82 v1 复审）：样本数据结构不固定，any[] → unknown[]
  sample_data: unknown[]
  status: 'active' | 'inactive'
  created_at: string
  updated_at: string
}

export interface ImportColumn {
  key: string
  label: string
  type: 'string' | 'number' | 'date' | 'boolean'
  required: boolean
  default_value?: any
  validation_rule?: string
}

export interface ImportTask {
  id: number
  task_code: string
  template_id: number
  template_name: string
  file_name: string
  file_path: string
  status: 'pending' | 'processing' | 'completed' | 'failed'
  total_rows: number
  processed_rows: number
  success_rows: number
  failed_rows: number
  error_log: string
  created_by: number
  created_by_name: string
  created_at: string
  completed_at: string
}

export function listImportTemplates(params?: QueryParams): Promise<ApiResponse<ImportTemplate[]>> {
  return request.get('/data-import/templates', { params })
}

export function getImportTemplate(id: number): Promise<ApiResponse<ImportTemplate>> {
  return request.get(`/data-import/templates/${id}`)
}

export function createImportTemplate(
  data: Partial<ImportTemplate>
): Promise<ApiResponse<ImportTemplate>> {
  return request.post('/data-import/templates', data)
}

export function updateImportTemplate(
  id: number,
  data: Partial<ImportTemplate>
): Promise<ApiResponse<ImportTemplate>> {
  return request.put(`/data-import/templates/${id}`, data)
}

export function deleteImportTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/data-import/templates/${id}`)
}

export function listImportTasks(params?: QueryParams): Promise<ApiResponse<ImportTask[]>> {
  return request.get('/data-import/tasks', { params })
}

export function getImportTask(id: number): Promise<ApiResponse<ImportTask>> {
  return request.get(`/data-import/tasks/${id}`)
}

export function uploadImportFile(templateId: number, file: File): Promise<ApiResponse<ImportTask>> {
  const formData = new FormData()
  formData.append('file', file)
  formData.append('template_id', templateId.toString())
  return request.post('/data-import/tasks', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  })
}

export function cancelImportTask(id: number): Promise<ApiResponse<void>> {
  return request.post(`/data-import/tasks/${id}/cancel`)
}

export function retryImportTask(id: number): Promise<ApiResponse<void>> {
  return request.post(`/data-import/tasks/${id}/retry`)
}

export function downloadImportTemplate(id: number): Promise<Blob> {
  return request.get(`/data-import/templates/${id}/download`, {
    responseType: 'blob',
  })
}

export function downloadErrorLog(taskId: number): Promise<Blob> {
  return request.get(`/data-import/tasks/${taskId}/error-log`, {
    responseType: 'blob',
  })
}
