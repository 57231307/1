import { request } from './request';

export interface Quality8dReport {
  id: number;
  quality_issue_id: number;
  status: string;
  plan?: string;
  closed_at?: string;
  closed_by?: number;
  created_at: string;
}

export interface StartEightdPayload {
  quality_issue_id: number;
  plan?: string;
}

export function startQuality8d(data: StartEightdPayload) {
  return request.post('/quality-8d-reports', data);
}

export function getQuality8dList(params?: Record<string, unknown>) {
  return request.get('/quality-8d-reports', { params });
}

export function getQuality8dDetail(id: number) {
  return request.get(`/quality-8d-reports/${id}`);
}

export function advanceQuality8d(id: number, data: Record<string, unknown>) {
  return request.post(`/quality-8d-reports/${id}/advance`, data);
}

export function closeQuality8d(id: number, data?: Record<string, unknown>) {
  return request.post(`/quality-8d-reports/${id}/close`, data ?? {});
}

export function printQuality8d(id: number) {
  return request.get(`/quality-8d-reports/${id}/print`);
}

export const QUALITY_8D_STAGES = [
  'not_started',
  'd0',
  'd1',
  'd2',
  'd3',
  'd4',
  'd5',
  'd6',
  'd7',
  'd8',
  'closed',
] as const;
