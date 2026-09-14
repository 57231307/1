import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

/**
 * 职业健康体检档案（occupational_health_exam）
 * 体检类型：pre_employment(上岗前) / in_service(在岗期间) / resignation(离岗时)
 */
export type HealthExamType = 'pre_employment' | 'in_service' | 'resignation';

/** 体检结果：normal(正常) / abnormal(异常) / contraindication(禁忌) */
export type HealthExamResult = 'normal' | 'abnormal' | 'contraindication';

export interface HealthExam {
  id: number;
  worker_id: number;
  exam_type: HealthExamType | string;
  exam_date: string;
  /** 下次体检日期（在岗期间体检必填，自动到期提醒） */
  next_exam_date: string | null;
  exam_organization: string | null;
  exam_result: HealthExamResult | string;
  /** 危害暴露史（JSON） */
  hazard_exposure: unknown;
  contraindications: string | null;
  report_url: string | null;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

export interface CreateHealthExamPayload {
  /** 工人 ID（必填） */
  worker_id: number;
  exam_type: HealthExamType;
  exam_date: string;
  /** 复查/下次体检日期（在岗期间体检用于到期提醒） */
  next_exam_date?: string;
  exam_organization?: string;
  exam_result: HealthExamResult;
  hazard_exposure?: Record<string, unknown>;
  contraindications?: string;
  report_url?: string;
  remarks?: string;
}

export interface HealthExamListQuery extends QueryParams {
  worker_id?: number;
  exam_type?: string;
  exam_result?: string;
}

export interface HealthExamListResult {
  list: HealthExam[];
  total: number;
}

export function getHealthExamList(
  params?: HealthExamListQuery
): Promise<ApiResponse<HealthExamListResult>> {
  return request.get('/occupational-health/health-exams', { params });
}

export function createHealthExam(
  data: CreateHealthExamPayload
): Promise<ApiResponse<HealthExam>> {
  return request.post('/occupational-health/health-exams', data);
}

/** 扫描体检到期预警（在岗期间体检按 next_exam_date 提前提醒） */
export function scanHealthExamExpiryWarnings(): Promise<ApiResponse<unknown>> {
  return request.post('/occupational-health/health-exams/scan-expiry-warnings');
}
