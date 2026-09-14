import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

/**
 * 劳动合同（labor_contract）状态
 * 后端状态机：active → expiring_soon → expired / terminated
 */
export type LaborContractStatus = 'active' | 'expiring_soon' | 'expired' | 'terminated';

/** 合同类型：fixed_term(固定期限) / permanent(无固定期限) / task_based(任务制) */
export type LaborContractType = 'fixed_term' | 'permanent' | 'task_based';

/** 工时制度：standard(标准) / comprehensive(综合) / flexible(不定) */
export type WorkingHoursSystem = 'standard' | 'comprehensive' | 'flexible';

export interface LaborContract {
  id: number;
  worker_id: number;
  contract_no: string;
  contract_type: LaborContractType | string;
  start_date: string;
  end_date: string | null;
  probation_end_date: string | null;
  probation_salary: string | number;
  regular_salary: string | number;
  position: string | null;
  department: string | null;
  work_location: string | null;
  working_hours_system: WorkingHoursSystem | string;
  sign_date: string;
  status: LaborContractStatus;
  termination_date: string | null;
  termination_reason: string | null;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

export interface CreateLaborContractPayload {
  /** 工人 ID（必填） */
  worker_id: number;
  /** 合同编号（全局唯一） */
  contract_no: string;
  contract_type: LaborContractType;
  start_date: string;
  /** 合同结束日期（无固定期限合同留空） */
  end_date?: string;
  /** 试用期结束日期（留空表示无试用期） */
  probation_end_date?: string;
  probation_salary: number;
  regular_salary: number;
  position?: string;
  department?: string;
  work_location?: string;
  working_hours_system: WorkingHoursSystem;
  sign_date: string;
  remarks?: string;
}

export interface UpdateLaborContractPayload {
  position?: string;
  department?: string;
  work_location?: string;
  working_hours_system?: WorkingHoursSystem;
  remarks?: string;
}

export interface TerminateLaborContractPayload {
  termination_date: string;
  termination_reason: string;
}

export type LaborContractExpiryLevel =
  | 'Normal'
  | 'Warning90Days'
  | 'Warning60Days'
  | 'Warning30Days'
  | 'Expired';

export interface LaborContractExpiryWarning {
  contract: LaborContract;
  level: LaborContractExpiryLevel;
  days_until_expiry: number;
}

export interface LaborContractListQuery extends QueryParams {
  worker_id?: number;
  contract_type?: string;
  status?: string;
}

export interface LaborContractListResult {
  list: LaborContract[];
  total: number;
}

export function getLaborContractList(
  params?: LaborContractListQuery
): Promise<ApiResponse<LaborContractListResult>> {
  return request.get('/labor-contracts', { params });
}

export function getLaborContract(id: number): Promise<ApiResponse<LaborContract>> {
  return request.get(`/labor-contracts/${id}`);
}

export function getActiveLaborContractByWorker(
  workerId: number
): Promise<ApiResponse<LaborContract>> {
  return request.get(`/labor-contracts/active-by-worker/${workerId}`);
}

export function createLaborContract(
  data: CreateLaborContractPayload
): Promise<ApiResponse<LaborContract>> {
  return request.post('/labor-contracts', data);
}

export function updateLaborContract(
  id: number,
  data: UpdateLaborContractPayload
): Promise<ApiResponse<LaborContract>> {
  return request.put(`/labor-contracts/${id}`, data);
}

export function terminateLaborContract(
  id: number,
  data: TerminateLaborContractPayload
): Promise<ApiResponse<LaborContract>> {
  return request.post(`/labor-contracts/${id}/terminate`, data);
}

/** 扫描合同到期预警（30/60/90 天三级） */
export function scanLaborContractExpiryWarnings(): Promise<
  ApiResponse<LaborContractExpiryWarning[]>
> {
  return request.post('/labor-contracts/scan-expiry-warnings');
}
