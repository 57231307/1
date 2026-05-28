import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface AccountSubject {
  id: number
  code: string
  name: string
  parent_id?: number
  level: number
  category: string
  direction: string
  is_leaf: boolean
  status: number
  created_at: string
  updated_at: string
  children?: AccountSubject[]
}

export interface AccountSubjectCreateRequest {
  code: string
  name: string
  parent_id?: number
  category: string
  direction: string
}

export interface AccountSubjectUpdateRequest {
  name?: string
  status?: number
}

export function listSubjects(params?: QueryParams): Promise<ApiResponse<AccountSubject[]>> {
  return request.get('/gl/subjects', { params })
}

export function getSubjectTree(): Promise<ApiResponse<AccountSubject[]>> {
  return request.get('/gl/subjects/tree')
}

export function getSubject(id: number): Promise<ApiResponse<AccountSubject>> {
  return request.get(`/gl/subjects/${id}`)
}

export function createSubject(
  data: AccountSubjectCreateRequest
): Promise<ApiResponse<AccountSubject>> {
  return request.post('/gl/subjects', data)
}

export function updateSubject(
  id: number,
  data: AccountSubjectUpdateRequest
): Promise<ApiResponse<AccountSubject>> {
  return request.put(`/gl/subjects/${id}`, data)
}

export function deleteSubject(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/gl/subjects/${id}`)
}

export interface Voucher {
  id: number
  voucher_no: string
  voucher_date: string
  period_id: number
  period_name?: string
  voucher_type: string
  entries: VoucherEntry[]
  total_debit: number
  total_credit: number
  status: string
  created_by: number
  created_by_name?: string
  created_at: string
  updated_at: string
}

export interface VoucherEntry {
  id: number
  subject_id: number
  subject_code: string
  subject_name: string
  debit: number
  credit: number
  summary: string
}

export interface VoucherCreateRequest {
  voucher_date: string
  voucher_type: string
  entries: {
    subject_id: number
    debit: number
    credit: number
    summary: string
  }[]
}

export function listVouchers(params?: QueryParams): Promise<ApiResponse<Voucher[]>> {
  return request.get('/gl/vouchers', { params })
}

export function getVoucher(id: number): Promise<ApiResponse<Voucher>> {
  return request.get(`/gl/vouchers/${id}`)
}

export function createVoucher(data: VoucherCreateRequest): Promise<ApiResponse<Voucher>> {
  return request.post('/gl/vouchers', data)
}

export function submitVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/gl/vouchers/${id}/submit`)
}

export function reviewVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/gl/vouchers/${id}/review`)
}

export function postVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/gl/vouchers/${id}/post`)
}
