import { request } from './request'

export interface VoucherEntry {
  id?: number
  account_subject_id: number
  account_subject_code?: string
  account_subject_name?: string
  debit_amount: number
  credit_amount: number
  description?: string
}

export interface VoucherEntity {
  id?: number
  voucher_no: string
  voucher_date: string
  period_id: number
  period_name?: string
  type: string
  status: string
  description?: string
  total_debit: number
  total_credit: number
  entries: VoucherEntry[]
  created_by?: number
  created_by_name?: string
  approved_by?: number
  approved_by_name?: string
  posted_by?: number
  posted_by_name?: string
  created_at?: string
  approved_at?: string
  posted_at?: string
}

export interface QueryParams {
  page?: number
  pageSize?: number
  voucher_no?: string
  voucher_date_start?: string
  voucher_date_end?: string
  type?: string
  status?: string
}

export function listVouchers(params?: QueryParams) {
  return request.get('/api/v1/vouchers', { params })
}

export function getVoucher(id: number) {
  return request.get(`/api/v1/vouchers/${id}`)
}

export function createVoucher(data: Partial<VoucherEntity>) {
  return request.post('/api/v1/vouchers', data)
}

export function updateVoucher(id: number, data: Partial<VoucherEntity>) {
  return request.put(`/api/v1/vouchers/${id}`, data)
}

export function deleteVoucher(id: number) {
  return request.delete(`/api/v1/vouchers/${id}`)
}

export function approveVoucher(id: number) {
  return request.patch(`/api/v1/vouchers/${id}/approve`)
}

export function postVoucher(id: number) {
  return request.patch(`/api/v1/vouchers/${id}/post`)
}

export function unpostVoucher(id: number) {
  return request.patch(`/api/v1/vouchers/${id}/unpost`)
}

export function getVoucherTypes() {
  return request.get('/api/v1/vouchers/types')
}

export function generateVoucherNo() {
  return request.get('/api/v1/vouchers/generate-no')
}