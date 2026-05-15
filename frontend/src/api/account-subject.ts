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
  return request.get('/account-subjects', { params })
}

export function getAccountSubject(id: number) {
  return request.get(`/account-subjects/${id}`)
}

export function createAccountSubject(data: Partial<AccountSubjectEntity>) {
  return request.post('/account-subjects', data)
}

export function updateAccountSubject(id: number, data: Partial<AccountSubjectEntity>) {
  return request.put(`/account-subjects/${id}`, data)
}

export function deleteAccountSubject(id: number) {
  return request.delete(`/account-subjects/${id}`)
}

export function enableAccountSubject(id: number) {
  return request.patch(`/account-subjects/${id}/enable`)
}

export function disableAccountSubject(id: number) {
  return request.patch(`/account-subjects/${id}/disable`)
}

export function getAccountSubjectTree() {
  return request.get('/account-subjects/tree')
}