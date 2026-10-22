import { request } from './request';

export interface LaborContract {
  id: number;
  status: string;
  [key: string]: unknown;
}

// 后端 CreateLaborContractRequest（services/labor_contract_service.rs）：
// worker_id/contract_no/contract_type/start_date/probation_salary/regular_salary/
// working_hours_system/sign_date 为非 Option 必填，其余可选；wire 为 snake_case。
// working_hours_system 取值 standard/comprehensive/flexible，contract_type 取值
// fixed_term/permanent/task_based，均由后端校验函数定义，前端不得自造词表。
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

// 后端 UpdateLaborContractRequest：全字段可选、缺省即不变更；
// worker_id/contract_no/sign_date 属身份与缔约事实列，更新端点不接收，
// 编辑提交必须走本类型（禁止回喂创建载荷整包导致多余键被静默丢弃）。
export interface UpdateLaborContractPayload {
  contract_type?: string;
  start_date?: string;
  end_date?: string;
  probation_end_date?: string;
  probation_salary?: number;
  regular_salary?: number;
  position?: string;
  department?: string;
  work_location?: string;
  working_hours_system?: string;
  remarks?: string;
}

export function getLaborContractList(params?: Record<string, unknown>) {
  return request.get('/labor-contracts', { params });
}

export function createLaborContract(data: CreateLaborContractPayload) {
  return request.post('/labor-contracts', data);
}

export function getLaborContract(id: number) {
  return request.get(`/labor-contracts/${id}`);
}

export function getActiveContractByWorker(workerId: number) {
  return request.get(`/labor-contracts/active-by-worker/${workerId}`);
}

export function updateLaborContract(id: number, data: UpdateLaborContractPayload) {
  return request.put(`/labor-contracts/${id}`, data);
}

export function terminateLaborContract(id: number, data: Record<string, unknown>) {
  return request.post(`/labor-contracts/${id}/terminate`, data);
}

export function scanContractExpiryWarnings() {
  return request.post('/labor-contracts/scan-expiry-warnings');
}

export function printLaborContract(id: number) {
  return request.get(`/labor-contracts/${id}/print`);
}
