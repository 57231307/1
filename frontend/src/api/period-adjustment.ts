import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 期末调整（period_adjustment_record）类型与状态
 * 类型：estimate 暂估 / amortization 摊销 / provision 预提 / transfer 转账
 * 状态机：draft → confirmed → reversed；draft → cancelled
 * 后端路由：routes/period_adjustment.rs（nest 到 /api/v1/erp/period-adjustments）
 */
/**
 * 调整类型。后端 create 校验仅接受 estimate / amortization / provision
 * （services/period_adjustment_service.rs validate_adjustment_type），故不含 transfer。
 */
export type PeriodAdjustmentType = 'estimate' | 'amortization' | 'provision';

export type PeriodAdjustmentStatus = 'draft' | 'confirmed' | 'reversed' | 'cancelled';

export interface PeriodAdjustment {
  id: number;
  adjustment_no: string;
  adjustment_type: PeriodAdjustmentType;
  /** 调整期间，如 2026-01 */
  period: string;
  description: string;
  debit_subject_code: string;
  debit_subject_name: string;
  credit_subject_code: string;
  credit_subject_name: string;
  amount: string | number;
  source_type: string | null;
  source_bill_id: number | null;
  source_bill_no: string | null;
  voucher_id: number | null;
  reverse_voucher_id: number | null;
  status: PeriodAdjustmentStatus;
  confirmed_by: number | null;
  confirmed_at: string | null;
  reversed_by: number | null;
  reversed_at: string | null;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * 创建期末调整载荷，字段严格对齐后端 CreatePeriodAdjustmentRequest
 * （services/period_adjustment_service.rs）。8 个字段为后端必填，均须由表单真实收集。
 * 注：字段之间不得插入行内注释——check-api-request 的 TS 解析按 `;`/`,` 切段，
 * 段首的注释会使紧随其后的字段被漏读。
 */
export interface CreatePeriodAdjustmentPayload {
  adjustment_type: PeriodAdjustmentType;
  period: string;
  description: string;
  debit_subject_code: string;
  debit_subject_name: string;
  credit_subject_code: string;
  credit_subject_name: string;
  amount: number;
  source_type?: string;
  source_bill_id?: number;
  source_bill_no?: string;
  remarks?: string;
}

export interface PeriodAdjustmentListQuery {
  adjustment_type?: string;
  period?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

export function getPeriodAdjustmentList(
  params?: PeriodAdjustmentListQuery
): Promise<ApiResponse<PaginatedResponse<PeriodAdjustment>>> {
  return request.get('/period-adjustments', { params });
}

export function getPeriodAdjustment(id: number): Promise<ApiResponse<PeriodAdjustment>> {
  return request.get(`/period-adjustments/${id}`);
}

export function createPeriodAdjustment(
  data: CreatePeriodAdjustmentPayload
): Promise<ApiResponse<PeriodAdjustment>> {
  return request.post('/period-adjustments', data);
}

/** 确认调整（draft → confirmed，生成调整凭证） */
export function confirmPeriodAdjustment(id: number): Promise<ApiResponse<PeriodAdjustment>> {
  return request.post(`/period-adjustments/${id}/confirm`);
}

/** 红字冲销（confirmed → reversed，生成红字凭证） */
export function reversePeriodAdjustment(id: number): Promise<ApiResponse<PeriodAdjustment>> {
  return request.post(`/period-adjustments/${id}/reverse`);
}

/** 取消调整（draft → cancelled） */
export function cancelPeriodAdjustment(id: number): Promise<ApiResponse<PeriodAdjustment>> {
  return request.post(`/period-adjustments/${id}/cancel`);
}
