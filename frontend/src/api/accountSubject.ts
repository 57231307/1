import { request } from './request'

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

export const listAccountSubjects = (params?: any) =>
  request.get('/account-subjects', { params })

export const getAccountSubject = (id: number) =>
  request.get(`/account-subjects/${id}`)

export const createAccountSubject = (data: Partial<AccountSubject>) =>
  request.post('/account-subjects', data)

export const updateAccountSubject = (id: number, data: Partial<AccountSubject>) =>
  request.put(`/account-subjects/${id}`, data)

export const deleteAccountSubject = (id: number) =>
  request.delete(`/account-subjects/${id}`)

export const getAccountSubjectTree = () =>
  request.get('/account-subjects/tree')
