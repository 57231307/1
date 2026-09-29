import { request } from './request';

export interface LaborContract {
  id: number;
  status: string;
  [key: string]: unknown;
}

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

export function updateLaborContract(id: number, data: Partial<CreateLaborContractPayload>) {
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
