import { request } from './request';
import type { ApiResponse } from '@/types/api';
import type { WaybillStatus } from '@/constants/waybill-status';

export interface LogisticsWaybill {
  id?: number;
  order_id?: number;
  /** 关联销售订单号：运单表只存 order_id，由列表/详情接口回查补齐 */
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
  /** 签收人 user_id，由签收接口从登录态写入 */
  signed_by?: number;
  /** 签收时间，同时是应收确认的时点 */
  signed_at?: string;
  sign_remark?: string;
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

/**
 * 运单更新入参（对齐 backend UpdateWaybillReq）：字段缺省表示不修改。
 * 运输信息仅在 IN_TRANSIT 阶段可改；status 仅支持 IN_TRANSIT → DELIVERED。
 */
export interface UpdateWaybillPayload {
  status?: WaybillStatus;
  logistics_company?: string;
  tracking_number?: string;
  driver_name?: string;
  driver_phone?: string;
  freight_fee?: number;
  /** 预计到达日期，格式 YYYY-MM-DD */
  expected_arrival?: string;
  notes?: string;
}

// D14 Batch 5b：原 logisticsApi.update 转为风格 B 函数（更新运单信息与状态）
export const updateLogistics = (id: number, data: UpdateWaybillPayload) =>
  request.put<ApiResponse<LogisticsWaybill>>(`/inventory/logistics/${id}`, data);

/** 电子签收入参（对齐 backend SignWaybillRequest），均为可选项 */
export interface SignWaybillPayload {
  /** 纸质回单扫描件 URL */
  receipt_url?: string;
  /** 现场签收照片 URL */
  photo_url?: string;
  /** 签收备注（异常说明） */
  remark?: string;
}

/**
 * 电子签收：DELIVERED → SIGNED，写入签收人与签收时间，
 * 并在同一事务内把关联销售订单的应收发票由草稿推进为已确认
 */
export const signWaybill = (id: number, data: SignWaybillPayload = {}) =>
  request.post<ApiResponse<LogisticsWaybill>>(`/inventory/logistics/${id}/sign`, data);

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
  request.get<ApiResponse<unknown[]>>(`/logistics-tracking/waybills/${waybillId}/tracking-events`);

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
