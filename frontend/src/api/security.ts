import { request } from './request'
import type { ApiResponse } from '@/types/api'

/** 账号锁定状态（来自后端 /api/v1/erp/lock-status） */
export interface LockStatus {
  user_id: number
  username: string
  is_locked: boolean
  failed_attempts: number
  locked_until: string | null
  max_attempts: number
}

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

// D14 Batch 5b：原 securityApi.getStats 转为风格 B 函数
// 后端路由 GET /api/v1/erp/stats
export const getSecurityStats = () =>
  request.get<ApiResponse<SecurityStats>>('/stats')

// D14 Batch 5b：原 securityApi.getLoginLogs 转为风格 B 函数
// 后端路由 GET /api/v1/erp/login-logs
export const getLoginLogList = (params?: SecurityQueryParams) =>
  request.get<ApiResponse<{ list: LoginLog[]; total: number }>>('/login-logs', {
    params,
  })

// D14 Batch 5b：原 securityApi.getLockedAccounts 转为风格 B 函数
// 后端路由 GET /api/v1/erp/locked-accounts
export const getLockedAccountList = () =>
  request.get<ApiResponse<LockedAccount[]>>('/locked-accounts')

// D14 Batch 5b：原 securityApi.unlockAccount 转为风格 B 函数
// 后端路由 POST /api/v1/erp/locked-accounts/:id/unlock
export const unlockAccount = (id: number) =>
  request.post<ApiResponse<void>>(`/locked-accounts/${id}/unlock`)

// D14 Batch 5b：原 securityApi.getSecurityAlerts 转为风格 B 函数
// 后端路由 GET /api/v1/erp/alerts
export const getSecurityAlertList = () =>
  request.get<ApiResponse<SecurityAlert[]>>('/alerts')

// D14 Batch 5b：原 securityApi.resolveAlert 转为风格 B 函数
// 后端路由 POST /api/v1/erp/alerts/:id/resolve
export const resolveSecurityAlert = (id: number) =>
  request.post<ApiResponse<void>>(`/alerts/${id}/resolve`)

// D14 Batch 5b：原 securityApi.exportLoginLogs 转为风格 B 函数
// 后端路由 GET /api/v1/erp/login-logs/export
export const exportLoginLogs = (params?: SecurityQueryParams) =>
  request.get<Blob>('/login-logs/export', { params, responseType: 'blob' })

// D14 Batch 5b：原 securityApi.checkLockStatus 转为风格 B 函数
/**
 * 检查指定用户名的账号锁定状态
 * 调 GET /api/v1/erp/lock-status?username=xxx
 * 用于登录页：用户输入用户名失焦时预检查 / 登录失败后展示锁定信息
 */
export const checkLockStatus = (username: string) =>
  request.get<ApiResponse<LockStatus>>('/lock-status', {
    params: { username },
  })
