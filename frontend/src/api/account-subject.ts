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

export interface QueryParams {
  page?: number
  pageSize?: number
  code?: string
  name?: string
  category?: string
  type?: string
  is_enabled?: boolean
}

export function listAccountSubjects(params?: QueryParams) {
  return request.get('/gl/subjects', { params })
}

export function getAccountSubject(id: number) {
  return request.get(`/gl/subjects/${id}`)
}

export function createAccountSubject(data: Partial<AccountSubjectEntity>) {
  return request.post('/gl/subjects', data)
}

export function updateAccountSubject(id: number, data: Partial<AccountSubjectEntity>) {
  return request.put(`/gl/subjects/${id}`, data)
}

export function deleteAccountSubject(id: number) {
  return request.delete(`/gl/subjects/${id}`)
}

export function enableAccountSubject(id: number) {
  return request.put(`/gl/subjects/${id}`, { is_enabled: true })
}

export function disableAccountSubject(id: number) {
  return request.put(`/gl/subjects/${id}`, { is_enabled: false })
}

export function getAccountSubjectTree() {
  return request.get('/gl/subjects/tree')
}
