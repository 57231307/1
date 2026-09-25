import { request } from './request';

export interface AccountSubjectEntity {
  id?: number;
  code: string;
  name: string;
  parent_id?: number;
  level: number;
  category: string;
  type: string;
  balance_type: string;
  description?: string;
  // account_subjects.status 为 VARCHAR（'active'/'inactive'）真实列，非布尔
  status: string;
  created_at?: string;
  updated_at?: string;
}

export function getAccountSubjectList(params?: Record<string, unknown>) {
  return request.get('/subjects', { params });
}

export function getAccountSubject(id: number) {
  return request.get(`/subjects/${id}`);
}

export function createAccountSubject(data: Partial<AccountSubjectEntity>) {
  return request.post('/subjects', data);
}

export function updateAccountSubject(id: number, data: Partial<AccountSubjectEntity>) {
  return request.put(`/subjects/${id}`, data);
}

export function deleteAccountSubject(id: number) {
  return request.delete(`/subjects/${id}`);
}

export function getAccountSubjectTree() {
  return request.get('/subjects/tree');
}
