import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 社保公积金缴纳记录（social_insurance_record）状态
 * 后端状态机：pending → paid / cancelled
 */
export type SocialInsuranceStatus = 'pending' | 'paid' | 'cancelled';

export interface SocialInsuranceRecord {
  id: number;
  worker_id: number;
  period_year: number;
  period_month: number;
  base_amount: string | number;
  pension_employer: string | number;
  pension_employee: string | number;
  medical_employer: string | number;
  medical_employee: string | number;
  unemployment_employer: string | number;
  unemployment_employee: string | number;
  work_injury_employer: string | number;
  maternity_employer: string | number;
  housing_fund_employer: string | number;
  housing_fund_employee: string | number;
  total_employer: string | number;
  total_employee: string | number;
  status: SocialInsuranceStatus;
  payment_date: string | null;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSocialInsurancePayload {
  /** 工人 ID（必填） */
  worker_id: number;
  /** 缴费周期-年 */
  period_year: number;
  /** 缴费周期-月（1-12） */
  period_month: number;
  /** 缴费基数（应为上年度月平均工资） */
  base_amount: number;
  payment_date?: string;
  remarks?: string;
}

export interface MarkSocialInsurancePaidPayload {
  payment_date: string;
}

export interface SocialInsuranceListQuery extends QueryParams {
  worker_id?: number;
  period_year?: number;
  period_month?: number;
  status?: string;
}

/**
 * 五险一金费率配置（对齐后端 SocialInsuranceService::InsuranceRateConfig 默认值）
 * 前端本地试算用；后端创建记录时会以相同默认费率自动计算落库
 */
export interface InsuranceRateConfig {
  pension_employer_rate: number;
  pension_employee_rate: number;
  medical_employer_rate: number;
  medical_employee_rate: number;
  unemployment_employer_rate: number;
  unemployment_employee_rate: number;
  work_injury_employer_rate: number;
  maternity_employer_rate: number;
  housing_fund_employer_rate: number;
  housing_fund_employee_rate: number;
  min_base_amount: number;
  max_base_amount: number;
}

export const DEFAULT_INSURANCE_RATE_CONFIG: InsuranceRateConfig = {
  pension_employer_rate: 0.16,
  pension_employee_rate: 0.08,
  medical_employer_rate: 0.08,
  medical_employee_rate: 0.02,
  unemployment_employer_rate: 0.005,
  unemployment_employee_rate: 0.005,
  work_injury_employer_rate: 0.004,
  maternity_employer_rate: 0.01,
  housing_fund_employer_rate: 0.12,
  housing_fund_employee_rate: 0.12,
  min_base_amount: 4250,
  max_base_amount: 31884,
};

/** 将缴费基数裁剪到当地上下限内 */
export function clampBaseAmount(base: number, config: InsuranceRateConfig): number {
  return Math.min(Math.max(base, config.min_base_amount), config.max_base_amount);
}

/** 前端按默认费率试算五险一金（单位/个人/合计） */
export function calculateInsurance(base: number, config: InsuranceRateConfig = DEFAULT_INSURANCE_RATE_CONFIG) {
  const clamped = clampBaseAmount(base, config);
  const items = [
    { key: 'pension', label: '养老保险', employer: config.pension_employer_rate, employee: config.pension_employee_rate },
    { key: 'medical', label: '医疗保险', employer: config.medical_employer_rate, employee: config.medical_employee_rate },
    { key: 'unemployment', label: '失业保险', employer: config.unemployment_employer_rate, employee: config.unemployment_employee_rate },
    { key: 'work_injury', label: '工伤保险', employer: config.work_injury_employer_rate, employee: 0 },
    { key: 'maternity', label: '生育保险', employer: config.maternity_employer_rate, employee: 0 },
    { key: 'housing_fund', label: '住房公积金', employer: config.housing_fund_employer_rate, employee: config.housing_fund_employee_rate },
  ].map(item => ({
    ...item,
    employer_amount: +(clamped * item.employer).toFixed(2),
    employee_amount: +(clamped * item.employee).toFixed(2),
  }));
  const total_employer = +items.reduce((sum, i) => sum + i.employer_amount, 0).toFixed(2);
  const total_employee = +items.reduce((sum, i) => sum + i.employee_amount, 0).toFixed(2);
  return { base_amount: clamped, items, total_employer, total_employee };
}

export function getSocialInsuranceList(
  params?: SocialInsuranceListQuery
): Promise<ApiResponse<PaginatedResponse<SocialInsuranceRecord>>> {
  return request.get('/social-insurance', { params });
}

export function getSocialInsurance(id: number): Promise<ApiResponse<SocialInsuranceRecord>> {
  return request.get(`/social-insurance/${id}`);
}

export function getSocialInsuranceByWorkerPeriod(
  workerId: number,
  params: { period_year: number; period_month: number }
): Promise<ApiResponse<SocialInsuranceRecord>> {
  return request.get(`/social-insurance/by-worker/${workerId}`, { params });
}

export function createSocialInsurance(
  data: CreateSocialInsurancePayload
): Promise<ApiResponse<SocialInsuranceRecord>> {
  return request.post('/social-insurance', data);
}

export function markSocialInsurancePaid(
  id: number,
  data: MarkSocialInsurancePaidPayload
): Promise<ApiResponse<SocialInsuranceRecord>> {
  return request.post(`/social-insurance/${id}/mark-paid`, data);
}

export function cancelSocialInsurance(id: number): Promise<ApiResponse<SocialInsuranceRecord>> {
  return request.post(`/social-insurance/${id}/cancel`);
}
