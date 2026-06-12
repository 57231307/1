import { request } from './request'

export interface AccountSubjectEntity {
  id?: number
  code: string
  name: string
  parent_id?: number
  level: number
  category: string
  type: string
  balance_type: string
  description?: string
  is_enabled: boolean
  created_at?: string
  updated_at?: string
}

export interface AccountSubject {
  id?: number
  subject_code: string
  subject_name: string
  subject_type: string
  parent_id?: number
  level: number
  balance_direction: string
  is_active: boolean
  created_at?: string
  updated_at?: string
}

export function listAccountSubjects(params?: any) {
  return request.get('/subjects', { params })
}

export function getAccountSubject(id: number) {
  return request.get(`/subjects/${id}`)
}

export function createAccountSubject(data: Partial<AccountSubjectEntity>) {
  return request.post('/subjects', data)
}

export function updateAccountSubject(id: number, data: Partial<AccountSubjectEntity>) {
  return request.put(`/subjects/${id}`, data)
}

export function deleteAccountSubject(id: number) {
  return request.delete(`/subjects/${id}`)
}

export function enableAccountSubject(id: number) {
  return request.put(`/subjects/${id}`, { is_enabled: true })
}

export function disableAccountSubject(id: number) {
  return request.put(`/subjects/${id}`, { is_enabled: false })
}

export function getAccountSubjectTree() {
  return request.get('/subjects/tree')
}
