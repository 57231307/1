import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 人力 / 环保合规域 API
 *
 * 人力合规（backend/src/routes/labor_contract.rs、occupational_health.rs，前缀 /api/v1/erp）：
 * - POST/GET /labor-contracts、GET/PUT /labor-contracts/{id}、POST /labor-contracts/{id}/terminate、
 *   POST /labor-contracts/scan-expiry-warnings
 * - POST/GET /occupational-health/hazard-monitorings
 *
 * 环保合规（backend/src/routes/pollution_permit.rs、pollution_monitoring.rs，前缀 /api/v1/erp）：
 * - POST/GET /pollution-permits、GET /pollution-permits/{id}、POST /pollution-permits/{id}/revoke
 * - POST/GET /pollution-monitoring/records、GET /pollution-monitoring/exceedance-alerts
 */

// ---------------- 劳动合同 ----------------

/** 劳动合同（对齐 backend/src/models/labor_contract.rs） */
export interface LaborContract {
  id: number;
  worker_id: number;
  contract_no: string;
  contract_type: string;
  start_date: string;
  end_date: string | null;
  probation_end_date: string | null;
  probation_salary: string | number;
  regular_salary: string | number;
  position: string | null;
  department: string | null;
  work_location: string | null;
  working_hours_system: string;
  sign_date: string;
  status: string;
  termination_date: string | null;
  termination_reason: string | null;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/** 创建劳动合同请求（对齐 CreateLaborContractRequest） */
export interface CreateLaborContractPayload {
  worker_id: number;
  contract_no: string;
  contract_type: string;
  start_date: string;
  end_date?: string;
  probation_end_date?: string;
  probation_salary: number;
  regular_salary: number;
  position?: string;
  department?: string;
  work_location?: string;
  working_hours_system: string;
  sign_date: string;
  remarks?: string;
}

/** 劳动合同状态 → 标签类型映射 */
export const laborContractStatusTagMap: Record<
  string,
  'info' | 'warning' | 'success' | 'danger'
> = {
  active: 'success',
  terminated: 'danger',
  expired: 'warning',
};

/** 获取劳动合同列表（后端返回 { list, total }） */
export function getLaborContractList(params?: {
  worker_id?: number;
  contract_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
}): Promise<ApiResponse<{ list: LaborContract[]; total: number }>> {
  return request.get('/labor-contracts', { params });
}

/** 创建劳动合同 */
export function createLaborContract(
  data: CreateLaborContractPayload
): Promise<ApiResponse<LaborContract>> {
  return request.post('/labor-contracts', data);
}

/** 终止劳动合同（仅 active 状态可终止） */
export function terminateLaborContract(
  id: number,
  data: { termination_date: string; termination_reason: string }
): Promise<ApiResponse<LaborContract>> {
  return request.post(`/labor-contracts/${id}/terminate`, data);
}

/** 扫描合同到期预警 */
export function scanLaborContractExpiryWarnings(): Promise<ApiResponse<unknown>> {
  return request.post('/labor-contracts/scan-expiry-warnings', {});
}

// ---------------- 职业危害因素检测（人力） ----------------

/** 职业危害因素检测记录（对齐 backend/src/models/occupational_hazard_monitoring.rs） */
export interface HazardMonitoring {
  id: number;
  hazard_type: string;
  hazard_name: string;
  monitoring_point: string;
  measured_value: string | number;
  unit: string;
  limit_value: string | number;
  is_exceeding: boolean;
  exceeding_ratio: string | number | null;
  monitoring_date: string;
  monitoring_organization: string | null;
  monitoring_method: string | null;
  report_url: string | null;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/** 创建职业危害因素检测记录请求（对齐 CreateHazardMonitoringRequest） */
export interface CreateHazardMonitoringPayload {
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

/** 获取职业危害因素检测记录列表（后端返回 { list, total }） */
export function getHazardMonitoringList(params?: {
  hazard_type?: string;
  hazard_name?: string;
  only_exceeding?: boolean;
  page?: number;
  page_size?: number;
}): Promise<ApiResponse<{ list: HazardMonitoring[]; total: number }>> {
  return request.get('/occupational-health/hazard-monitorings', { params });
}

/** 创建职业危害因素检测记录 */
export function createHazardMonitoring(
  data: CreateHazardMonitoringPayload
): Promise<ApiResponse<HazardMonitoring>> {
  return request.post('/occupational-health/hazard-monitorings', data);
}

// ---------------- 排污许可证（环保） ----------------

/** 排污许可证（对齐 backend/src/models/pollution_permit.rs） */
export interface PollutionPermit {
  id: number;
  permit_no: string;
  permit_type: string;
  permit_category: string | null;
  issue_date: string;
  expiry_date: string;
  issuing_authority: string;
  permitted_capacity: string | number | null;
  capacity_unit: string | null;
  permitted_pollutants: unknown;
  status: string;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/** 创建排污许可证请求（对齐 CreatePollutionPermitRequest） */
export interface CreatePollutionPermitPayload {
  permit_no: string;
  permit_type: string;
  permit_category?: string;
  issue_date: string;
  expiry_date: string;
  issuing_authority: string;
  permitted_capacity?: number;
  capacity_unit?: string;
  remarks?: string;
}

/** 排污许可证状态 → 标签类型映射 */
export const pollutionPermitStatusTagMap: Record<
  string,
  'info' | 'warning' | 'success' | 'danger'
> = {
  active: 'success',
  revoked: 'danger',
  expired: 'warning',
};

/** 获取排污许可证列表（后端返回 { items, total }） */
export function getPollutionPermitList(params?: {
  permit_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
}): Promise<ApiResponse<{ items: PollutionPermit[]; total: number }>> {
  return request.get('/pollution-permits', { params });
}

/** 创建排污许可证 */
export function createPollutionPermit(
  data: CreatePollutionPermitPayload
): Promise<ApiResponse<PollutionPermit>> {
  return request.post('/pollution-permits', data);
}

/** 撤销排污许可证 */
export function revokePollutionPermit(id: number): Promise<ApiResponse<PollutionPermit>> {
  return request.post(`/pollution-permits/${id}/revoke`, {});
}

// ---------------- 污染物监测记录（环保） ----------------

/** 污染物监测记录（对齐 backend/src/models/pollutant_monitoring_record.rs） */
export interface PollutantMonitoringRecord {
  id: number;
  monitoring_type: string;
  monitoring_point: string;
  pollutant_name: string;
  measured_value: string | number;
  unit: string;
  limit_value: string | number;
  is_exceeding: boolean;
  exceeding_ratio: string | number | null;
  monitoring_time: string;
  monitoring_method: string | null;
  equipment_id: number | null;
  operator_id: number | null;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

/** 创建污染物监测记录请求（对齐 CreateMonitoringRecordRequest） */
export interface CreateMonitoringRecordPayload {
  monitoring_type: string;
  monitoring_point: string;
  pollutant_name: string;
  measured_value: number;
  unit: string;
  limit_value: number;
  monitoring_time: string;
  monitoring_method?: string;
  equipment_id?: number;
  operator_id?: number;
  remarks?: string;
}

/** 获取污染物监测记录列表（后端返回 { items, total }） */
export function getPollutantMonitoringRecordList(params?: {
  monitoring_type?: string;
  pollutant_name?: string;
  only_exceeding?: boolean;
  page?: number;
  page_size?: number;
}): Promise<ApiResponse<{ items: PollutantMonitoringRecord[]; total: number }>> {
  return request.get('/pollution-monitoring/records', { params });
}

/** 创建污染物监测记录 */
export function createPollutantMonitoringRecord(
  data: CreateMonitoringRecordPayload
): Promise<ApiResponse<PollutantMonitoringRecord>> {
  return request.post('/pollution-monitoring/records', data);
}

/** 扫描超标告警 */
export function scanExceedanceAlerts(): Promise<ApiResponse<unknown>> {
  return request.get('/pollution-monitoring/exceedance-alerts');
}
