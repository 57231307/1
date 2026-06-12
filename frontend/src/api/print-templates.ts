import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface PrintTemplate {
  id: number
  template_code: string
  template_name: string
  description: string
  module: 'sales' | 'purchase' | 'inventory' | 'finance' | 'production' | 'logistics'
  type: 'order' | 'invoice' | 'receipt' | 'label' | 'report' | 'custom'
  paper_size: 'A4' | 'A5' | 'B5' | 'Letter' | 'Custom'
  orientation: 'portrait' | 'landscape'
  content: string
  css_styles: string
  variables: Record<string, any>
  status: 'active' | 'inactive'
  is_default: boolean
  created_by: number
  created_by_name: string
  created_at: string
  updated_at: string
}

export function listPrintTemplates(params?: QueryParams): Promise<ApiResponse<PrintTemplate[]>> {
  return request.get('/print-templates', { params })
}

export function getPrintTemplate(id: number): Promise<ApiResponse<PrintTemplate>> {
  return request.get(`/print-templates/${id}`)
}

export function createPrintTemplate(
  data: Partial<PrintTemplate>
): Promise<ApiResponse<PrintTemplate>> {
  return request.post('/print-templates', data)
}

export function updatePrintTemplate(
  id: number,
  data: Partial<PrintTemplate>
): Promise<ApiResponse<PrintTemplate>> {
  return request.put(`/print-templates/${id}`, data)
}

export function deletePrintTemplate(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/print-templates/${id}`)
}

export function previewPrintTemplate(
  id: number,
  data?: Record<string, any>
): Promise<ApiResponse<any>> {
  return request.post(`/print-templates/${id}/preview`, data)
}

export function printTemplate(id: number, data: Record<string, any>): Promise<void> {
  return request.post(`/print-templates/${id}/print`, data)
}

export function setDefaultPrintTemplate(id: number): Promise<ApiResponse<void>> {
  return request.put(`/print-templates/${id}/set-default`)
}

export function copyPrintTemplate(id: number): Promise<ApiResponse<PrintTemplate>> {
  return request.post(`/print-templates/${id}/copy`)
}
