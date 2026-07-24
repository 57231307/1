import { request } from './request'
import type { ApiResponse } from '@/types/api'

/** 运单状态联合类型 */
export type WaybillStatus = 'pending' | 'shipped' | 'in_transit' | 'delivered' | 'cancelled'

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
  status: WaybillStatus
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

// D14 Batch 5b：原 logisticsApi.list 转为风格 B 函数（运单列表）
export const getLogisticsList = (params?: LogisticsQueryParams) =>
  request.get<ApiResponse<{ list: LogisticsWaybill[]; total: number }>>('/inventory/logistics', {
    params,
  })

// D14 Batch 5b：原 logisticsApi.create 转为风格 B 函数（创建运单）
export const createLogistics = (data: Partial<LogisticsWaybill>) =>
  request.post<ApiResponse<LogisticsWaybill>>('/inventory/logistics', data)

// D14 Batch 5b：原 logisticsApi.getById 转为风格 B 函数（获取运单详情）
export const getLogisticsById = (id: number) =>
  request.get<ApiResponse<LogisticsWaybill>>(`/inventory/logistics/${id}`)

// D14 Batch 5b：原 logisticsApi.update 转为风格 B 函数（更新运单状态）
export const updateLogistics = (id: number, data: Partial<LogisticsWaybill>) =>
  request.put<ApiResponse<LogisticsWaybill>>(`/inventory/logistics/${id}`, data)

// D14 Batch 5b：原 logisticsApi.delete 转为风格 B 函数（删除运单）
export const deleteLogistics = (id: number) =>
  request.delete<ApiResponse<void>>(`/inventory/logistics/${id}`)
