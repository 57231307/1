import { request } from './request'
import type { ApiResponse } from './request'

export interface LogisticsInfo {
  id: number
  waybill_no: string
  order_id: number
  order_no: string
  logistics_company?: string
  tracking_no?: string
  sender_name: string
  sender_address: string
  sender_phone: string
  receiver_name: string
  receiver_address: string
  receiver_phone: string
  shipping_date?: string
  estimated_delivery_date?: string
  actual_delivery_date?: string
  status: string
  current_location?: string
  remark?: string
}

export interface Waybill {
  id: number
  waybill_no: string
  order_id: number
  order_no: string
  logistics_company: string
  tracking_no: string
  sender_name: string
  sender_address: string
  sender_phone: string
  receiver_name: string
  receiver_address: string
  receiver_phone: string
  shipping_date: string
  status: string
  remark?: string
}

export const logisticsApi = {
  listWaybills: (params?: any) =>
    request.get<ApiResponse<{ list: Waybill[]; total: number }>>('/logistics', { params }),

  createWaybill: (data: Partial<Waybill>) =>
    request.post<ApiResponse<Waybill>>('/logistics', data),

  getWaybill: (id: number) =>
    request.get<ApiResponse<Waybill>>(`/logistics/${id}`),

  updateWaybillStatus: (id: number, data: { status: string; current_location?: string }) =>
    request.put<ApiResponse<Waybill>>(`/logistics/${id}`, data),

  deleteWaybill: (id: number) =>
    request.delete<ApiResponse<null>>(`/logistics/${id}`),

  scanToShip: (data: { barcode: string; quantity?: number }) =>
    request.post<ApiResponse<any>>('/scanner/scan-to-ship', data),
}
