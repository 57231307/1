import { request } from './request';
import type { ApiResponse } from '@/types/api';

/** 排污许可证（对齐 backend/src/services/pollution_permit_service.rs） */
export interface PollutionPermit {
  id: number;
  permit_no: string;
  permit_type: string;
  status: string;
  [key: string]: unknown;
}

export interface CreatePollutionPermitPayload {
  permit_type: string;
  [key: string]: unknown;
}


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
