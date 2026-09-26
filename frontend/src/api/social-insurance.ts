import { request } from './request';

export interface SocialInsurance {
  id: number;
  status: string;
  [key: string]: unknown;
}

export interface CreateSocialInsurancePayload {
  worker_id: number;
  period_year: number;
  period_month: number;
  base_amount: number;
  payment_date?: string;
  remarks?: string;
}

export function getSocialInsuranceList(params?: Record<string, unknown>) {
  return request.get('/social-insurance', { params });
}

export function createSocialInsurance(data: CreateSocialInsurancePayload) {
  return request.post('/social-insurance', data);
}

export function getSocialInsurance(id: number) {
  return request.get(`/social-insurance/${id}`);
}

export function getInsuranceByWorkerPeriod(
  workerId: number,
  params: { period_year: number; period_month: number }
) {
  return request.get(`/social-insurance/by-worker/${workerId}`, { params });
}

export function markInsurancePaid(id: number, data?: Record<string, unknown>) {
  return request.post(`/social-insurance/${id}/mark-paid`, data ?? {});
}

export function cancelInsurance(id: number) {
  return request.post(`/social-insurance/${id}/cancel`);
}

export const INSURANCE_STATUS_LABEL: Record<string, string> = {
  pending: '未缴',
  paid: '已缴',
  cancelled: '已取消',
};
