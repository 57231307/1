import { request } from './request';

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

export interface CreateHealthExamPayload {
  worker_id: number;
  exam_type: string;
  exam_date: string;
  next_exam_date?: string;
  exam_organization?: string;
  exam_result: string;
  hazard_exposure?: Record<string, unknown>;
  contraindications?: string;
}

export function getHealthExamList(params?: Record<string, unknown>) {
  return request.get('/occupational-health/health-exams', { params });
}

export function createHealthExam(data: CreateHealthExamPayload) {
  return request.post('/occupational-health/health-exams', data);
}

export function scanExamExpiryWarnings() {
  return request.post('/occupational-health/health-exams/scan-expiry-warnings');
}

export function getHazardMonitoringList(params?: Record<string, unknown>) {
  return request.get('/occupational-health/hazard-monitorings', { params });
}

export function createHazardMonitoring(data: Record<string, unknown>) {
  return request.post('/occupational-health/hazard-monitorings', data);
}

export function getPpeDistributionList(params?: Record<string, unknown>) {
  return request.get('/occupational-health/ppe-distributions', { params });
}

export function createPpeDistribution(data: Record<string, unknown>) {
  return request.post('/occupational-health/ppe-distributions', data);
}

export function scanPpeExpired() {
  return request.post('/occupational-health/ppe-distributions/scan-expired');
}
