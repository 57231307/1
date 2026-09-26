import { request } from './request';

export interface WageRate {
  id: number;
  status: string;
  [key: string]: unknown;
}

export interface WageRecord {
  id: number;
  status: string;
  [key: string]: unknown;
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
}

export function getWageRateList(params?: Record<string, unknown>) {
  return request.get('/production/wage-rates', { params });
}

export function createWageRate(data: CreateWageRatePayload) {
  return request.post('/production/wage-rates', data);
}

export function updateWageRate(id: number, data: Partial<CreateWageRatePayload>) {
  return request.put(`/production/wage-rates/${id}`, data);
}

export function deleteWageRate(id: number) {
  return request.delete(`/production/wage-rates/${id}`);
}

export function activateWageRate(id: number) {
  return request.post(`/production/wage-rates/${id}/activate`);
}

export function disableWageRate(id: number) {
  return request.post(`/production/wage-rates/${id}/disable`);
}

export function getEffectiveWageRate(routeId: number, date: string) {
  return request.get(`/production/wage-rates/effective/${routeId}`, { params: { date } });
}

export interface CreateWageRecordPayload {
  period_start: string;
  period_end: string;
  workshop?: string;
  remarks?: string;
}

export function getWageRecordList(params?: Record<string, unknown>) {
  return request.get('/production/wage-records', { params });
}

export function createWageRecord(data: CreateWageRecordPayload) {
  return request.post('/production/wage-records', data);
}

export function updateWageRecord(id: number, data: { workshop?: string; remarks?: string }) {
  return request.put(`/production/wage-records/${id}`, data);
}

export function deleteWageRecord(id: number) {
  return request.delete(`/production/wage-records/${id}`);
}

export function calculateWageRecord(id: number, data?: Record<string, unknown>) {
  return request.post(`/production/wage-records/${id}/calculate`, data ?? {});
}

export function confirmWageRecord(id: number) {
  return request.post(`/production/wage-records/${id}/confirm`);
}

export function payWageRecord(id: number) {
  return request.post(`/production/wage-records/${id}/pay`);
}

export function cancelWageRecord(id: number) {
  return request.post(`/production/wage-records/${id}/cancel`);
}

export function getWageRecordDetails(id: number) {
  return request.get(`/production/wage-records/${id}/details`);
}

export function exportWageRecordDetails(id: number) {
  return request.get(`/production/wage-records/${id}/details/export`, { responseType: 'blob' });
}

export function getWageDetailsByWorker(workerId: number) {
  return request.get(`/production/wage-details/by-worker/${workerId}`);
}

export const WAGE_RECORD_STATUS_LABEL: Record<string, string> = {
  draft: '草稿',
  calculated: '已核算',
  confirmed: '已确认',
  paid: '已发放',
  cancelled: '已取消',
};
