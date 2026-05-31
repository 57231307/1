import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface LogisticsWaybill {
  id?: number
  waybill_no: string
  order_id?: number
  order_no?: string
  logistics_company: string
  tracking_number: string
  driver_name?: string
  driver_phone?: string
  freight_fee?: number
  expected_arrival?: string
  actual_arrival?: string
  status: 'pending' | 'shipped' | 'in_transit' | 'delivered' | 'cancelled'
  notes?: string
  created_at?: string
  updated_at?: string
}

export interface LogisticsQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  status?: string
  logistics_company?: string
  order_id?: number
  start_date?: string
  end_date?: string
}

export const logisticsApi = {
  // 运单列表
  list: (params?: LogisticsQueryParams) =>
    request.get<ApiResponse<{ list: LogisticsWaybill[]; total: number }>>('/logistics', { params }),

  // 创建运单
  create: (data: Partial<LogisticsWaybill>) =>
    request.post<ApiResponse<LogisticsWaybill>>('/logistics', data),

  // 获取运单详情
  getById: (id: number) =>
    request.get<ApiResponse<LogisticsWaybill>>(`/logistics/${id}`),

  // 更新运单状态
  update: (id: number, data: Partial<LogisticsWaybill>) =>
    request.put<ApiResponse<LogisticsWaybill>>(`/logistics/${id}`, data),

  // 删除运单
  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/logistics/${id}`),
}
