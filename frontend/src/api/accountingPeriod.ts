import { request } from './request'

export interface AccountingPeriodEntity {
  id?: number
  name: string
  year: number
  month: number
  start_date: string
  end_date: string
  status: string
  closed_at?: string
  created_at?: string
  updated_at?: string
}

export interface QueryParams {
  page?: number
  pageSize?: number
  year?: number
  month?: number
  status?: string
}

export function listAccountingPeriods(params?: QueryParams) {
  return request.get('/api/v1/accounting-periods', { params })
}

export function getAccountingPeriod(id: number) {
  return request.get(`/api/v1/accounting-periods/${id}`)
}

export function createAccountingPeriod(data: Partial<AccountingPeriodEntity>) {
  return request.post('/api/v1/accounting-periods', data)
}

export function updateAccountingPeriod(id: number, data: Partial<AccountingPeriodEntity>) {
  return request.put(`/api/v1/accounting-periods/${id}`, data)
}

export function deleteAccountingPeriod(id: number) {
  return request.delete(`/api/v1/accounting-periods/${id}`)
}

export function closePeriod(id: number) {
  return request.patch(`/api/v1/accounting-periods/${id}/close`)
}

export function reopenPeriod(id: number) {
  return request.patch(`/api/v1/accounting-periods/${id}/reopen`)
}

export function getCurrentPeriod() {
  return request.get('/api/v1/accounting-periods/current')
}

export function getPeriodByDate(date: string) {
  return request.get('/api/v1/accounting-periods/by-date', { params: { date } })
}