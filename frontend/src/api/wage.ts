import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 工价方案状态（后端状态机）：draft → active → disabled
 */
export type WageRateStatus = 'draft' | 'active' | 'disabled';

export interface WageRate {
  id: number;
  rate_no: string;
  process_route_id: number;
  route_code: string;
  route_name: string;
  wage_type: string;
  piece_price: string | number;
  time_price: string | number;
  grade_a_ratio: string | number;
  grade_b_ratio: string | number;
  grade_c_ratio: string | number;
  effective_date: string;
  expiry_date: string | null;
  workshop: string | null;
  status: WageRateStatus;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

export interface ListWageRatesQuery extends QueryParams {
  route_code?: string;
  process_route_id?: number;
  workshop?: string;
  status?: string;
}

export interface CreateWageRatePayload {
  process_route_id: number;
  wage_type?: string;
  piece_price?: number;
  time_price?: number;
  grade_a_ratio?: number;
  grade_b_ratio?: number;
  grade_c_ratio?: number;
  effective_date: string;
  expiry_date?: string;
  workshop?: string;
  remarks?: string;
}

/**
 * 工资记录状态（后端状态机）：draft → confirmed → paid；cancelled 为异常终止
 */
export type WageRecordStatus = 'draft' | 'confirmed' | 'paid' | 'cancelled';

export interface WageRecord {
  id: number;
  record_no: string;
  period_start: string;
  period_end: string;
  workshop: string | null;
  total_workers: number;
  total_step_records: number;
  total_qualified_quantity: string | number;
  total_duration_minutes: number;
  total_amount: string | number;
  status: WageRecordStatus;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

export interface ListWageRecordsQuery extends QueryParams {
  record_no?: string;
  workshop?: string;
  status?: string;
  period_start?: string;
  period_end?: string;
}

export interface CreateWageRecordPayload {
  period_start: string;
  period_end: string;
  workshop?: string;
  remarks?: string;
}

// ==================== 工价方案 ====================

export function getWageRates(
  params?: ListWageRatesQuery
): Promise<ApiResponse<PaginatedResponse<WageRate>>> {
  return request.get('/production/wage-rates', { params });
}

export function createWageRate(data: CreateWageRatePayload): Promise<ApiResponse<WageRate>> {
  return request.post('/production/wage-rates', data);
}

/** 启用工价（draft → active） */
export function activateWageRate(id: number): Promise<ApiResponse<WageRate>> {
  return request.post(`/production/wage-rates/${id}/activate`);
}

/** 停用工价（active → disabled） */
export function disableWageRate(id: number): Promise<ApiResponse<WageRate>> {
  return request.post(`/production/wage-rates/${id}/disable`);
}

// ==================== 工资记录 ====================

export function getWageRecords(
  params?: ListWageRecordsQuery
): Promise<ApiResponse<PaginatedResponse<WageRecord>>> {
  return request.get('/production/wage-records', { params });
}

export function createWageRecord(data: CreateWageRecordPayload): Promise<ApiResponse<WageRecord>> {
  return request.post('/production/wage-records', data);
}

/** 触发工资计算（draft 内，可重算） */
export function calculateWage(
  id: number,
  recalculate?: boolean
): Promise<ApiResponse<WageRecord>> {
  return request.post(`/production/wage-records/${id}/calculate`, { recalculate });
}

/** 确认工资记录（draft → confirmed） */
export function confirmWageRecord(id: number): Promise<ApiResponse<WageRecord>> {
  return request.post(`/production/wage-records/${id}/confirm`);
}

/** 发放工资（confirmed → paid） */
export function payWageRecord(id: number): Promise<ApiResponse<WageRecord>> {
  return request.post(`/production/wage-records/${id}/pay`);
}

/** 取消工资记录 */
export function cancelWageRecord(id: number): Promise<ApiResponse<WageRecord>> {
  return request.post(`/production/wage-records/${id}/cancel`);
}
