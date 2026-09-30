import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface HealthExam {
  id: number;
  [key: string]: unknown;
}

export interface HazardMonitoring {
  id: number;
  [key: string]: unknown;
}

export interface PpeDistribution {
  id: number;
  [key: string]: unknown;
}

/**
 * 本域三个列表端点出参信封为 {list,total}（occupational_health_handler.rs:36/:60/:94
 * json!({"list":..,"total":..})），不是 PaginatedResponse 的 items 形状，也没有 page 回显。
 */
export interface ListEnvelope<T> {
  list: T[];
  total: number;
}

/** 对齐后端 CreateHealthExamRequest（services/occupational_health_service.rs:51-68）；created_by 由后端从会话注入，前端不发送 */
export interface CreateHealthExamPayload {
  worker_id: number;
  /** 词表：pre_employment / in_service / resignation（service :566-573） */
  exam_type: string;
  exam_date: string;
  next_exam_date?: string;
  exam_organization?: string;
  /** 词表：normal / abnormal / contraindication（service :575-584） */
  exam_result: string;
  hazard_exposure?: unknown[];
  contraindications?: string;
  report_url?: string;
  remarks?: string;
}

/** 对齐后端 CreateHazardMonitoringRequest（service :31-48）；前 7 键均为必填（非 Option），created_by 由后端注入 */
export interface CreateHazardMonitoringPayload {
  /** 词表：chemical / physical / dust / biological（service :554-561） */
  hazard_type: string;
  hazard_name: string;
  monitoring_point: string;
  measured_value: number;
  unit: string;
  limit_value: number;
  monitoring_date: string;
  monitoring_organization?: string;
  monitoring_method?: string;
  report_url?: string;
  remarks?: string;
}

/** 对齐后端 CreatePpeDistributionRequest（service :71-85）；ppe_type/distribution_date 为必填，created_by 由后端注入 */
export interface CreatePpeDistributionPayload {
  worker_id: number;
  ppe_name: string;
  /** 词表：mask / gloves / goggles / earplug / respirator / suit（service :587-594） */
  ppe_type: string;
  specification?: string;
  quantity: number;
  distribution_date: string;
  expiry_date?: string;
  hazard_type?: string;
  remarks?: string;
}

export function getHealthExamList(params?: Record<string, unknown>) {
  return request.get<ApiResponse<ListEnvelope<HealthExam>>>('/occupational-health/health-exams', {
    params,
  });
}

export function createHealthExam(data: CreateHealthExamPayload) {
  return request.post<ApiResponse<HealthExam>>('/occupational-health/health-exams', data);
}

/** data 为裸数组 Vec<ExamExpiryWarning>（handler :65-72） */
export function scanExamExpiryWarnings() {
  return request.post<ApiResponse<Array<Record<string, unknown>>>>(
    '/occupational-health/health-exams/scan-expiry-warnings'
  );
}

export function getHazardMonitoringList(params?: Record<string, unknown>) {
  return request.get<ApiResponse<ListEnvelope<HazardMonitoring>>>(
    '/occupational-health/hazard-monitorings',
    { params }
  );
}

export function createHazardMonitoring(data: CreateHazardMonitoringPayload) {
  return request.post<ApiResponse<HazardMonitoring>>(
    '/occupational-health/hazard-monitorings',
    data
  );
}

export function getPpeDistributionList(params?: Record<string, unknown>) {
  return request.get<ApiResponse<ListEnvelope<PpeDistribution>>>(
    '/occupational-health/ppe-distributions',
    { params }
  );
}

export function createPpeDistribution(data: CreatePpeDistributionPayload) {
  return request.post<ApiResponse<PpeDistribution>>('/occupational-health/ppe-distributions', data);
}

/** data 为裸数组 Vec<PpeModel>（handler :110-117），即本次被置为 expired 的记录集合 */
export function scanPpeExpired() {
  return request.post<ApiResponse<PpeDistribution[]>>(
    '/occupational-health/ppe-distributions/scan-expired'
  );
}
