import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

// 批次 98 P2-D 修复（v5 复审）：原 type: any 改为 Element Plus Tag type 联合类型
export type FundAccountStatusType = 'success' | 'info' | 'warning' | 'danger' | 'primary'

export const FUND_ACCOUNT_STATUS: Record<string, { label: string; type: FundAccountStatusType }> = {
  active: { label: '正常', type: 'success' },
  inactive: { label: '停用', type: 'info' },
  frozen: { label: '冻结', type: 'warning' },
}

export interface FundAccount {
  id: number
  account_no?: string
  account_code?: string
  account_name: string
  account_type: string
  balance?: number
  current_balance?: number
  frozen_balance?: number
  available_balance?: number
  status: string
  bank_name?: string
  bank_account?: string
  remark?: string
  created_at?: string
}

export interface FundTransferRecord {
  id: number
  transfer_no: string
  from_account_id: number
  from_account_name?: string
  to_account_id: number
  to_account_name?: string
  amount: number
  status: string
  remark?: string
  created_at: string
}

export function getFundAccountList(
  params?: QueryParams
): Promise<ApiResponse<{ list: FundAccount[]; total: number }>> {
  return request.get('/fund-management/accounts', { params })
}

export function getFundAccount(id: number): Promise<ApiResponse<FundAccount>> {
  return request.get(`/fund-management/accounts/${id}`)
}

export function createFundAccount(data: Partial<FundAccount>): Promise<ApiResponse<FundAccount>> {
  return request.post('/fund-management/accounts', data)
}

export function updateFundAccount(
  id: number,
  data: Partial<FundAccount>
): Promise<ApiResponse<FundAccount>> {
  return request.put(`/fund-management/accounts/${id}`, data)
}

export function depositFund(
  id: number,
  amount: number,
  remark?: string
): Promise<ApiResponse<void>> {
  return request.post(`/fund-management/accounts/${id}/deposit`, { amount, remark })
}

export function withdrawFund(
  id: number,
  amount: number,
  remark?: string
): Promise<ApiResponse<void>> {
  return request.post(`/fund-management/accounts/${id}/withdraw`, { amount, remark })
}

export function freezeFund(id: number, amount: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/fund-management/accounts/${id}/freeze`, { amount, reason })
}

export function unfreezeFund(id: number, amount: number): Promise<ApiResponse<void>> {
  return request.post(`/fund-management/accounts/${id}/unfreeze`, { amount })
}

export function deleteFundAccount(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/fund-management/accounts/${id}`)
}

export function transferFund(data: {
  from_account_id: number
  to_account_id: number
  amount: number
  remark?: string
}): Promise<ApiResponse<void>> {
  return request.post('/fund-management/transfer', data)
}

export function getFundTransferList(
  params?: QueryParams
): Promise<ApiResponse<FundTransferRecord[]>> {
  return request.get('/fund-management/transfers', { params })
}

export function getFundTransfer(id: number): Promise<ApiResponse<FundTransferRecord>> {
  return request.get(`/fund-management/transfers/${id}`)
}
