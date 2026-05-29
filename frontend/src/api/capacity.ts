import { request } from './request'
import type { ApiResponse, PageResult } from '@/types/api'

export interface WorkCenter {
  id: number
  name: string
  code: string
  capacity_hours: number
  used_hours: number
  load_rate: number
  status: 'normal' | 'busy' | 'overload'
  bottleneck: boolean
}

export interface CapacityTrend {
  date: string
  planned_hours: number
  actual_hours: number
  capacity_hours: number
}

export interface CapacitySummary {
  total_work_centers: number
  normal_count: number
  busy_count: number
  overload_count: number
  bottleneck_count: number
  avg_load_rate: number
}

export const capacityApi = {
  getSummary: () => request.get<ApiResponse<CapacitySummary>>('/capacity/summary'),

  getTrend: (params?: { days?: number; work_center_id?: number }) =>
    request.get<ApiResponse<CapacityTrend[]>>('/capacity/trend', { params }),

  listWorkCenters: (params?: { page?: number; page_size?: number; status?: string }) =>
    request.get<ApiResponse<PageResult<WorkCenter>>>('/capacity/work-centers', { params }),

  getBottlenecks: () => request.get<ApiResponse<WorkCenter[]>>('/capacity/bottlenecks'),
}
