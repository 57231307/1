import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface FundAccount {
  id: number
  account_no: string
  account_name: string
  bank_name?: string
  current_balance: number
  status: 'active' | 'inactive' | 'frozen'
  remark?: string
  created_at?: string
  updated_at?: string
}

export const FUND_ACCOUNT_STATUS = {
  active: { label: '启用', type: 'success' },
  inactive: { label: '停用', type: 'info' },
  frozen: { label: '冻结', type: 'warning' },
}

export function listFundAccounts(params?: QueryParams): Promise<ApiResponse<{ list: FundAccount[]; total: number }>> {
  return request.get('/api/v1/erp/fund-management/accounts', { params })
}

export function getFundAccount(id: number): Promise<ApiResponse<FundAccount>> {
  return request.get(`/api/v1/erp/fund-management/accounts/${id}`)
}

export function createFundAccount(data: Partial<FundAccount>): Promise<ApiResponse<FundAccount>> {
  return request.post('/api/v1/erp/fund-management/accounts', data)
}

export function updateFundAccount(id: number, data: Partial<FundAccount>): Promise<ApiResponse<FundAccount>> {
  return request.put(`/api/v1/erp/fund-management/accounts/${id}`, data)
}

export function depositFund(id: number, amount: number, remark?: string): Promise<ApiResponse<void>> {
  return request.post(`/api/v1/erp/fund-management/accounts/${id}/deposit`, { amount, remark })
}

export function withdrawFund(id: number, amount: number, remark?: string): Promise<ApiResponse<void>> {
  return request.post(`/api/v1/erp/fund-management/accounts/${id}/withdraw`, { amount, remark })
}

export function transferFund(from_id: number, to_id: number, amount: number, remark?: string): Promise<ApiResponse<void>> {
  return request.post('/api/v1/erp/fund-management/transfer', { from_id, to_id, amount, remark })
}
