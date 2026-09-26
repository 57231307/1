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

/**
 * 推进请求（对齐 backend/src/models/quality_8d_dto.rs AdvanceStepPayload，
 * serde(tag = "step", rename_all = "snake_case")——每条推进边有不同必填字段）
 */
export type AdvanceEightdPayload =
  | { step: 'd1_team'; team_members: string }
  | { step: 'd2_problem'; problem_description: string }
  | { step: 'd3_interim'; interim_action: string }
  | { step: 'd4_root_cause'; method: '5why' | 'fishbone'; detail: string; summary: string }
  | { step: 'd5_permanent'; permanent_action: string; action_owner: string; due_date: string }
  | { step: 'd6_verify'; verification_result: string }
  | { step: 'd7_prevent'; prevention_action: string }
  | { step: 'd8_recognize'; closure_summary: string };

export function startQuality8d(data: StartEightdPayload) {
  return request.post('/quality-8d-reports', data);
}

export function getQuality8dList(params?: Record<string, unknown>) {
  return request.get('/quality-8d-reports', { params });
}

export function getQuality8dDetail(id: number) {
  return request.get(`/quality-8d-reports/${id}`);
}

export function advanceQuality8d(id: number, data: AdvanceEightdPayload) {
  return request.post(`/quality-8d-reports/${id}/advance`, data);
}

export function closeQuality8d(id: number, data?: Record<string, unknown>) {
  return request.post(`/quality-8d-reports/${id}/close`, data ?? {});
}

export function printQuality8d(id: number) {
  return request.get(`/quality-8d-reports/${id}/print`);
}

/**
 * 8D 阶段状态值。必须与后端 `EightDStatus::as_str()` 完全一致
 * （services/quality_8d_service.rs:85-99）：此前这里写的是 d0~d8，
 * 后端返回的却是 d0_plan/d1_team/...，列表按状态取推进边时全部落空，
 * 「推进下一阶段」对任何真实报告都提示无可推进的边，8D 流程在界面上走不动。
 */
export const QUALITY_8D_STAGES = [
  'not_started',
  'd0_plan',
  'd1_team',
  'd2_problem',
  'd3_interim',
  'd4_root_cause',
  'd5_permanent',
  'd6_verify',
  'd7_prevent',
  'd8_recognize',
  'closed',
] as const;
