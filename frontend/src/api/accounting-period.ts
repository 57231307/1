import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 会计期间详情/列表项（对齐后端 missing_handlers::AccountingPeriodDto，
 * 由 accounting_period::Model 逐字段转换；后端不存在 name/month/status 小写词表形态）。
 * 状态词表（写入方 backend/src/services/accounting_period_service.rs +
 * models/status/finance.rs）：OPEN / CLOSING / CLOSED（全大写）。
 */
export interface AccountingPeriodDetail {
  id: number;
  year: number;
  period: number;
  period_name: string;
  start_date: string;
  end_date: string;
  /** OPEN 可录入凭证 / CLOSING 关账中 / CLOSED 已关账 */
  status: 'OPEN' | 'CLOSING' | 'CLOSED' | string;
  closed_at: string | null;
  closed_by: number | null;
  created_at: string;
}

// 后端 missing_handlers::get_accounting_periods 签名无 Query<T>，任何查询参数都会被 Axum 丢弃，
// 故前端不发送 params（列表为全量返回，年度过滤只能在前端本地做）。
export function getAccountingPeriodList() {
  return request.get<ApiResponse<AccountingPeriodDetail[]>>('/finance/accounting-periods');
}

export function getAccountingPeriod(id: number) {
  return request.get<ApiResponse<AccountingPeriodDetail>>(`/finance/accounting-periods/${id}`);
}

/**
 * 创建期间请求（对齐后端 missing_handlers::CreateAccountingPeriodPayload）
 * 路由：POST /api/v1/erp/finance/accounting-periods
 * year/period 均非 Option 必填；period 带 #[validate(range(min=1,max=12))] 且 handler 调用
 * payload.validate()。起止日期由后端按 year/period 推导，前端不传（传了也会被 serde 丢弃）。
 */
export interface CreateAccountingPeriodPayload {
  /** 预算年度（必填） */
  year: number;
  /** 期间序号 1-12（必填） */
  period: number;
}

/**
 * 创建会计期间（年度 + 期间序号）
 */
export function createAccountingPeriod(data: CreateAccountingPeriodPayload) {
  return request.post('/finance/accounting-periods', data);
}

/**
 * 更新期间请求（对齐后端 missing_handlers::UpdateAccountingPeriodPayload）。
 * 后端仅支持改 period_name 与 status；period_name 为 Option<String> 且带
 * #[validate(length(min=1,max=50))]，空串会被 validator 判 422 —— 未填写时必须省略该键。
 * status 仅接受 OPEN / CLOSING（CLOSED 期间后端整体拒绝修改）。
 * year/period/起止日期不可通过本端点修改。
 */
export interface UpdateAccountingPeriodPayload {
  period_name?: string;
  status?: string;
}

export function updateAccountingPeriod(id: number, data: UpdateAccountingPeriodPayload) {
  return request.put(`/finance/accounting-periods/${id}`, data);
}

export function deleteAccountingPeriod(id: number) {
  return request.delete(`/finance/accounting-periods/${id}`);
}

export function closePeriod(id: number) {
  return request.post(`/finance/accounting-periods/${id}/close`);
}

/**
 * 反结账（POST /finance/accounting-periods/{id}/reopen）。
 * 后端 accounting_period_handler::reopen_period 接收 Json<ReopenPeriodRequest>，
 * reason 为非 Option String 必填（缺失即 422），必须由界面真实采集，不得塞默认值。
 */
export function reopenPeriod(id: number, reason: string) {
  return request.post(`/finance/accounting-periods/${id}/reopen`, { reason });
}

/** 当前开放期间（后端 get_current_period 无 Query<T>，任何参数都会被 Axum 丢弃） */
export function getCurrentPeriod() {
  return request.get<ApiResponse<AccountingPeriodDetail | null>>(
    '/finance/accounting-periods/current'
  );
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
export function initPeriod(): Promise<ApiResponse<AccountingPeriodDetail>> {
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
