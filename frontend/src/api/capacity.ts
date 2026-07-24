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

// D14 Batch 5b：原 capacityApi.getSummary 转为风格 B 函数
export const getCapacitySummary = () =>
  request.get<ApiResponse<CapacitySummary>>('/capacity/summary')

// D14 Batch 5b：原 capacityApi.getTrend 转为风格 B 函数
export const getCapacityTrend = (params?: { days?: number; work_center_id?: number }) =>
  request.get<ApiResponse<CapacityTrend[]>>('/capacity/trend', { params })

// D14 Batch 5b：原 capacityApi.listWorkCenters 转为风格 B 函数
export const getWorkCenterList = (params?: {
  page?: number
  page_size?: number
  status?: string
}) => request.get<ApiResponse<PageResult<WorkCenter>>>('/capacity/work-centers', { params })

// D14 Batch 5b：原 capacityApi.getBottlenecks 转为风格 B 函数
export const getCapacityBottlenecks = () =>
  request.get<ApiResponse<WorkCenter[]>>('/capacity/bottlenecks')
