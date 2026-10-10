import { request } from './request';
import type { ApiResponse } from '@/types/api';
import type { AxiosRequestConfig } from 'axios';

/** 账号锁定状态（来自后端 /api/v1/erp/lock-status） */
export interface LockStatus {
  user_id: number;
  username: string;
  is_locked: boolean;
  failed_attempts: number;
  locked_until: string | null;
  max_attempts: number;
}

export interface SecurityStats {
  todayLogins: number;
  todayFailures: number;
  lockedAccounts: number;
  securityAlerts: number;
}

export interface LoginLog {
  id: number;
  // 出参唯一真相：login_security_handler::LoginLogItem（Option 列序列化为 null，不得省略声明）
  user_id: number | null;
  username: string;
  login_type: string;
  ip_address: string;
  user_agent: string | null;
  status: string;
  fail_reason: string | null;
  login_time: string;
}

export interface LockedAccount {
  id: number;
  username: string;
  lock_reason: string;
  locked_at: string;
  unlock_at?: string;
}

export interface SecurityAlert {
  id: number;
  alert_type: string;
  username: string;
  ip_address: string;
  description: string;
  created_at: string;
  status: string;
}

/**
 * 登录日志查询/导出共用参数（唯一真相：login_security_handler::LoginLogQuery，
 * 全 Option → 可选）。后端无日期区间字段（date_range 传入即被 Axum 静默丢弃，
 * 时间范围过滤属后端缺口，已登记串行清单），故不再声明 date_range。
 * 导出端点固定取前 10000 条，page/page_size 仅对列表生效。
 */
export interface SecurityQueryParams {
  user_id?: number;
  username?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

// D14 Batch 5b：原 securityApi.getStats 转为风格 B 函数
// 后端路由 GET /api/v1/erp/stats
export const getSecurityStats = () => request.get<ApiResponse<SecurityStats>>('/stats');

// D14 Batch 5b：原 securityApi.getLoginLogs 转为风格 B 函数
// 后端路由 GET /api/v1/erp/login-logs。
// 注意：该端点分页承载键为 "list"（后端 json!({list,total,page,page_size}) 手拼，
// 偏离统一 PaginatedResponse{items}——后端信封漂移已登记串行清单，读取处按真实键取）。
export const getLoginLogList = (params?: SecurityQueryParams) =>
  request.get<ApiResponse<{ list: LoginLog[]; total: number; page: number; page_size: number }>>(
    '/login-logs',
    {
      params,
    }
  );

// D14 Batch 5b：原 securityApi.getLockedAccounts 转为风格 B 函数
// 后端路由 GET /api/v1/erp/locked-accounts
export const getLockedAccountList = () =>
  request.get<ApiResponse<LockedAccount[]>>('/locked-accounts');

// D14 Batch 5b：原 securityApi.unlockAccount 转为风格 B 函数
// 后端路由 POST /api/v1/erp/locked-accounts/:id/unlock
export const unlockAccount = (id: number) =>
  request.post<ApiResponse<void>>(`/locked-accounts/${id}/unlock`);

// D14 Batch 5b：原 securityApi.getSecurityAlerts 转为风格 B 函数
// 后端路由 GET /api/v1/erp/alerts
export const getSecurityAlertList = () => request.get<ApiResponse<SecurityAlert[]>>('/alerts');

// D14 Batch 5b：原 securityApi.resolveAlert 转为风格 B 函数
// 后端路由 POST /api/v1/erp/alerts/:id/resolve
export const resolveSecurityAlert = (id: number) =>
  request.post<ApiResponse<void>>(`/alerts/${id}/resolve`);

// D14 Batch 5b：原 securityApi.exportLoginLogs 转为风格 B 函数
// 后端路由 GET /api/v1/erp/login-logs/export
export const exportLoginLogs = (params?: SecurityQueryParams) =>
  request.get<Blob>('/login-logs/export', { params, responseType: 'blob' });

// D14 Batch 5b：原 securityApi.checkLockStatus 转为风格 B 函数
/**
 * 检查指定用户名的账号锁定状态
 * 调 GET /api/v1/erp/lock-status?username=xxx
 * 用于登录页：用户输入用户名失焦时预检查 / 登录失败后展示锁定信息
 */
export const checkLockStatus = (username: string) =>
  request.get<ApiResponse<LockStatus>>('/lock-status', {
    params: { username },
    timeout: 5000,
    _skipAuthRetry: true,
  } as AxiosRequestConfig & { _skipAuthRetry?: boolean });
