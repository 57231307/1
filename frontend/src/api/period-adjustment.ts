import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 期末调整（period_adjustment_record）类型与状态
 * 类型：estimate 暂估 / amortization 摊销 / provision 预提 / transfer 转账
 * 状态机：draft → confirmed → reversed；draft → cancelled
 * 后端路由：routes/period_adjustment.rs（nest 到 /api/v1/erp/period-adjustments）
 */
export type PeriodAdjustmentType = 'estimate' | 'amortization' | 'provision' | 'transfer';

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

export interface CreatePeriodAdjustmentPayload {
  /** 调整类型（必填） */
  adjustment_type: PeriodAdjustmentType;
  /** 调整期间，如 2026-01（必填） */
  period: string;
  /** 调整说明（必填） */
  description: string;
  /** 借方科目编码（必填） */
  debit_subject_code: string;
  /** 借方科目名称（必填） */
  debit_subject_name: string;
  /** 贷方科目编码（必填） */
  credit_subject_code: string;
  /** 贷方科目名称（必填） */
  credit_subject_name: string;
  /** 调整金额（必填） */
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
