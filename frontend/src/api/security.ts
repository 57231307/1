import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface SecurityStats {
  todayLogins: number
  todayFailures: number
  lockedAccounts: number
  securityAlerts: number
}

export interface LoginLog {
  id: number
  username: string
  login_type: string
  ip_address: string
  user_agent: string
  status: string
  fail_reason?: string
  login_time: string
}

export interface LockedAccount {
  id: number
  username: string
  lock_reason: string
  locked_at: string
  unlock_at?: string
}

export interface SecurityAlert {
  id: number
  alert_type: string
  username: string
  ip_address: string
  description: string
  created_at: string
  status: string
}

export interface SecurityQueryParams {
  page?: number
  page_size?: number
  username?: string
  status?: string
  date_range?: string[]
}

export const securityApi = {
  getStats: () => request.get<ApiResponse<SecurityStats>>('/security/stats'),

  getLoginLogs: (params?: SecurityQueryParams) =>
    request.get<ApiResponse<{ list: LoginLog[]; total: number }>>('/security/login-logs', {
      params,
    }),

  getLockedAccounts: () => request.get<ApiResponse<LockedAccount[]>>('/security/locked-accounts'),

  unlockAccount: (id: number) =>
    request.post<ApiResponse<void>>(`/security/locked-accounts/${id}/unlock`),

  getSecurityAlerts: () => request.get<ApiResponse<SecurityAlert[]>>('/security/alerts'),

  resolveAlert: (id: number) => request.post<ApiResponse<void>>(`/security/alerts/${id}/resolve`),

  exportLoginLogs: (params?: SecurityQueryParams) =>
    request.get<Blob>('/security/login-logs/export', { params, responseType: 'blob' }),
}
