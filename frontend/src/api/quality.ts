import { request } from './request'
import type { ApiResponse } from './request'

export interface QualityInspection {
  id: number
  inspection_no: string
  inspection_type: string
  product_id: number
  product_name: string
  quantity: number
  qualified_quantity: number
  unqualified_quantity: number
  inspector: string
  inspection_date: string
  result: string
  defect_count?: number
  defect_rate?: number
  remark?: string
  items?: QualityInspectionItem[]
}

export interface QualityInspectionItem {
  id: number
  inspection_id: number
  defect_type: string
  defect_count: number
  defect_description?: string
}

export interface QualityStandard {
  id: number
  standard_code: string
  standard_name: string
  standard_type: string
  version: string
  content?: string
  status: string
  effective_date?: string
  created_at?: string
}

export interface QualityDefect {
  id: number
  defect_code: string
  defect_name: string
  defect_type: string
  severity: string
  processing_method?: string
  status: string
}

export const qualityApi = {
  listStandards: (params?: any) =>
    request.get<ApiResponse<{ list: QualityStandard[]; total: number }>>('/quality-standards', { params }),

  createStandard: (data: Partial<QualityStandard>) =>
    request.post<ApiResponse<QualityStandard>>('/quality-standards', data),

  getStandard: (id: number) =>
    request.get<ApiResponse<QualityStandard>>(`/quality-standards/${id}`),

  updateStandard: (id: number, data: Partial<QualityStandard>) =>
    request.put<ApiResponse<QualityStandard>>(`/quality-standards/${id}`, data),

  deleteStandard: (id: number) =>
    request.delete<ApiResponse<null>>(`/quality-standards/${id}`),

  approveStandard: (id: number) =>
    request.post<ApiResponse<null>>(`/quality-standards/${id}/approve`),

  publishStandard: (id: number) =>
    request.post<ApiResponse<null>>(`/quality-standards/${id}/publish`),

  listInspectionRecords: (params?: any) =>
    request.get<ApiResponse<{ list: QualityInspection[]; total: number }>>('/quality-inspection/records', { params }),

  createInspectionRecord: (data: Partial<QualityInspection>) =>
    request.post<ApiResponse<QualityInspection>>('/quality-inspection/records', data),

  getInspectionRecord: (id: number) =>
    request.get<ApiResponse<QualityInspection>>(`/quality-inspection/records/${id}`),

  listDefects: (params?: any) =>
    request.get<ApiResponse<{ list: QualityDefect[]; total: number }>>('/quality-inspection/defects', { params }),

  processDefect: (id: number, data: { processing_method: string }) =>
    request.post<ApiResponse<null>>(`/quality-inspection/defects/${id}/process`, data),
}
