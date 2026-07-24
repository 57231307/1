import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

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

export function getVoucherList(params?: QueryParams): Promise<ApiResponse<VoucherEntity[]>> {
  return request.get('/vouchers', { params })
}

export function getVoucher(id: number): Promise<ApiResponse<VoucherEntity>> {
  return request.get(`/vouchers/${id}`)
}

export function createVoucher(data: Partial<VoucherEntity>): Promise<ApiResponse<VoucherEntity>> {
  return request.post('/vouchers', data)
}

export function updateVoucher(
  id: number,
  data: Partial<VoucherEntity>
): Promise<ApiResponse<VoucherEntity>> {
  return request.put(`/vouchers/${id}`, data)
}

export function deleteVoucher(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/vouchers/${id}`)
}

export function approveVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/review`)
}

export function postVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/post`)
}

export function unpostVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/unpost`)
}

export function getVoucherTypes(): Promise<ApiResponse<string[]>> {
  return request.get('/vouchers/types')
}

export function generateVoucherNo(): Promise<ApiResponse<{ voucher_no: string }>> {
  return request.get('/vouchers/generate-no')
}
