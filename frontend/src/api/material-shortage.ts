import { request } from './request'
import type { ApiResponse, PageResult } from '@/types/api'

export interface MaterialShortage {
  id: number
  material_code: string
  material_name: string
  spec: string
  current_stock: number
  required_quantity: number
  shortage_quantity: number
  unit: string
  expected_date: string
  source_type: 'production' | 'sales' | 'purchase'
  source_no: string
  status: 'pending' | 'notified' | 'resolved'
  severity: 'low' | 'medium' | 'high' | 'critical'
}

export interface MaterialShortageSummary {
  total_shortage_count: number
  critical_count: number
  high_count: number
  medium_count: number
  low_count: number
  total_shortage_amount: number
  last_check_time: string
}

export const materialShortageApi = {
  getSummary: () => request.get<ApiResponse<MaterialShortageSummary>>('/material-shortage/summary'),

  listShortages: (params?: {
    page?: number
    page_size?: number
    severity?: string
    status?: string
  }) =>
    request.get<ApiResponse<PageResult<MaterialShortage>>>('/material-shortage/list', { params }),

  triggerCheck: () =>
    request.post<ApiResponse<{ check_id: number; message: string }>>('/material-shortage/check'),

  updateStatus: (id: number, status: string) =>
    request.put<ApiResponse<void>>(`/material-shortage/${id}/status`, { status }),
}
