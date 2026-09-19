import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface AccountingPeriodEntity {
  id?: number;
  name: string;
  year: number;
  month: number;
  start_date: string;
  end_date: string;
  status: string;
  closed_at?: string;
  created_at?: string;
  updated_at?: string;
}

export function getAccountingPeriodList(params?: Record<string, unknown>) {
  return request.get<ApiResponse<AccountingPeriodDetail[]>>('/finance/accounting-periods', {
    params,
  });
}

export function getAccountingPeriod(id: number) {
  return request.get<ApiResponse<AccountingPeriodDetail>>(`/finance/accounting-periods/${id}`);
}

export function createAccountingPeriod(data: Partial<AccountingPeriodEntity>) {
  return request.post('/finance/accounting-periods', data);
}

/**
 * 创建期间请求（对齐后端 missing_handlers::CreateAccountingPeriodPayload）
 * 路由：POST /api/v1/erp/finance/accounting-periods
 */
export interface CreateAccountingPeriodPayload {
  /** 预算年度（必填） */
  year: number;
  /** 期间序号 1-12（必填） */
  period: number;
}

/** 会计期间详情（对齐后端 missing_handlers::AccountingPeriodDto） */
export interface AccountingPeriodDetail {
  id: number;
  year: number;
  period: number;
  period_name: string;
  start_date: string;
  end_date: string;
  /** OPEN 可录入凭证 / CLOSED 已关账 */
  status: 'OPEN' | 'CLOSED' | string;
  closed_at: string | null;
  closed_by: number | null;
  created_at: string;
}

/** 按年度+期间序号创建会计期间（强类型封装） */
export function createPeriodByYearPeriod(data: CreateAccountingPeriodPayload) {
  return request.post('/finance/accounting-periods', data);
}

export function updateAccountingPeriod(id: number, data: Partial<AccountingPeriodEntity>) {
  return request.put(`/finance/accounting-periods/${id}`, data);
}

export function deleteAccountingPeriod(id: number) {
  return request.delete(`/finance/accounting-periods/${id}`);
}

export function closePeriod(id: number) {
  return request.post(`/finance/accounting-periods/${id}/close`);
}

export function reopenPeriod(id: number) {
  return request.post(`/finance/accounting-periods/${id}/reopen`);
}

export function getCurrentPeriod() {
  return request.get('/finance/accounting-periods/current');
}

export function getPeriodByDate(date: string) {
  return request.get('/finance/accounting-periods/current', { params: { date } });
}

// ============== 期间初始化/年度结账（Batch 补齐 API 封装）==============

/** 年度结账结果（对应后端 accounting_period_service.rs::year_end_closing 返回 json） */
export interface YearEndClosingResult {
  year: number;
  next_year: number;
  transferred_subjects: number;
  retained_earnings_adjustment: number;
  operator_id: number;
  operated_at: string;
}

/**
 * 初始化当前财务期间（不存在时创建当年当月期间）
 * 后端路由：POST /api/v1/erp/finance/accounting-periods/init（routes/finance.rs accounting_period_routes）
 */
export function initPeriod(): Promise<ApiResponse<AccountingPeriodEntity>> {
  return request.post('/finance/accounting-periods/init');
}

/**
 * 年度结账（要求该年度 12 个期间全部 CLOSED；结转损益并创建下一年 1 月期间）
 * 后端路由：POST /api/v1/erp/finance/accounting-periods/year-end-closing?year=（routes/finance.rs accounting_period_routes）
 */
export function yearEndClosing(year: number): Promise<ApiResponse<YearEndClosingResult>> {
  return request.post('/finance/accounting-periods/year-end-closing', null, {
    params: { year },
  });
}
