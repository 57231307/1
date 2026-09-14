import { request } from './request';
import type { ApiResponse } from '@/types/api';

/** 运单状态联合类型 */
export type WaybillStatus = 'pending' | 'shipped' | 'in_transit' | 'delivered' | 'cancelled';

export interface LogisticsWaybill {
  id?: number;
  waybill_no: string;
  order_id?: number;
  order_no?: string;
  logistics_company: string;
  tracking_number: string;
  driver_name?: string;
  driver_phone?: string;
  freight_fee?: number;
  expected_arrival?: string;
  actual_arrival?: string;
  status: WaybillStatus;
  notes?: string;
  created_at?: string;
  updated_at?: string;
}

export interface LogisticsQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  status?: string;
  logistics_company?: string;
  order_id?: number;
  start_date?: string;
  end_date?: string;
}

// D14 Batch 5b：原 logisticsApi.list 转为风格 B 函数（运单列表）
export const getLogisticsList = (params?: LogisticsQueryParams) =>
  request.get<ApiResponse<{ items: LogisticsWaybill[]; total: number }>>('/inventory/logistics', {
    params,
  });

// D14 Batch 5b：原 logisticsApi.create 转为风格 B 函数（创建运单）
export const createLogistics = (data: Partial<LogisticsWaybill>) =>
  request.post<ApiResponse<LogisticsWaybill>>('/inventory/logistics', data);

// D14 Batch 5b：原 logisticsApi.getById 转为风格 B 函数（获取运单详情）
export const getLogisticsById = (id: number) =>
  request.get<ApiResponse<LogisticsWaybill>>(`/inventory/logistics/${id}`);

// D14 Batch 5b：原 logisticsApi.update 转为风格 B 函数（更新运单状态）
export const updateLogistics = (id: number, data: Partial<LogisticsWaybill>) =>
  request.put<ApiResponse<LogisticsWaybill>>(`/inventory/logistics/${id}`, data);

// D14 Batch 5b：原 logisticsApi.delete 转为风格 B 函数（删除运单）
export const deleteLogistics = (id: number) =>
  request.delete<ApiResponse<void>>(`/inventory/logistics/${id}`);

// ==================== 物流跟踪轨迹（/logistics-tracking 域） ====================

/** 物流跟踪事件（对齐后端 TrackingEvent） */
export interface TrackingEventPayload {
  /** 事件发生时间（RFC3339，如 2026-01-01T08:00:00Z） */
  event_time: string;
  location?: string;
  description: string;
  /** 事件类型：pickup / in_transit / arrived / delivered 等 */
  event_type: string;
  data_source?: string;
}

/** 查询运单轨迹事件（GET /logistics-tracking/waybills/{id}/tracking-events） */
export const getTrackingEvents = (waybillId: number) =>
  request.get<ApiResponse<unknown[]>>(
    `/logistics-tracking/waybills/${waybillId}/tracking-events`
  );

/** 记录运单轨迹事件（POST /logistics-tracking/waybills/{id}/tracking-events） */
export const recordTrackingEvent = (waybillId: number, data: TrackingEventPayload) =>
  request.post<ApiResponse<unknown>>(
    `/logistics-tracking/waybills/${waybillId}/tracking-events`,
    data
  );

/** 运单关联采购订单（POST /logistics-tracking/waybills/{id}/link-purchase-order） */
export const linkPurchaseOrder = (waybillId: number, data: { po_id: number }) =>
  request.post<ApiResponse<unknown>>(
    `/logistics-tracking/waybills/${waybillId}/link-purchase-order`,
    data
  );
