import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface BillingPlan {
  id?: number
  code: string
  name: string
  description?: string
  max_users: number
  max_storage_mb: number
  max_api_calls_per_day: number
  price_monthly: number
  price_yearly: number
  features?: string[]
  is_active?: boolean
  created_at?: string
}

export interface CurrentPlanInfo {
  plan_id: number
  plan_name: string
  plan_code: string
  billing_cycle: string
  start_date: string
  end_date: string
  auto_renew: boolean
  amount: number
}

export interface UsageStats {
  current_users: number
  max_users: number
  storage_used_mb: number
  max_storage_mb: number
  api_calls_today: number
  max_api_calls_per_day: number
  user_usage_percent: number
  storage_usage_percent: number
  api_usage_percent: number
}

export interface Invoice {
  id?: number
  invoice_no: string
  amount: number
  status: string
  billing_cycle: string
  paid_at?: string
  created_at?: string
}

export interface TenantConfig {
  id?: number
  key: string
  value: string
  config_type: string
  description?: string
  created_at?: string
  updated_at?: string
}

export const tenantBillingApi = {
  // 套餐管理
  getCurrentPlan: () => request.get<ApiResponse<CurrentPlanInfo>>('/tenants/billing/plan'),

  upgradePlan: (data: { plan_id: number; billing_cycle: string }) =>
    request.post<ApiResponse<CurrentPlanInfo>>('/tenants/billing/upgrade', data),

  getUsage: () => request.get<ApiResponse<UsageStats>>('/tenants/billing/usage'),

  getInvoices: (params?: { page?: number; page_size?: number }) =>
    request.get<ApiResponse<{ list: Invoice[]; total: number }>>('/tenants/billing/invoices', {
      params,
    }),

  renew: (data: { billing_cycle: string }) =>
    request.post<ApiResponse<CurrentPlanInfo>>('/tenants/billing/renew', data),

  // 套餐列表
  getPlans: () => request.get<ApiResponse<BillingPlan[]>>('/tenants/config/plans'),

  getPlanById: (id: number) => request.get<ApiResponse<BillingPlan>>(`/tenants/config/plans/${id}`),

  createPlan: (data: Partial<BillingPlan>) =>
    request.post<ApiResponse<BillingPlan>>('/tenants/config/plans', data),

  // 租户配置
  getConfigs: (params?: { key?: string; config_type?: string }) =>
    request.get<ApiResponse<TenantConfig[]>>('/tenants/config/settings', { params }),

  setConfig: (data: Partial<TenantConfig>) =>
    request.post<ApiResponse<void>>('/tenants/config/settings', data),

  deleteConfig: (key: string) =>
    request.delete<ApiResponse<void>>(`/tenants/config/settings/${key}`),

  // 使用统计
  getUsageStatistics: () => request.get<ApiResponse<UsageStats>>('/tenants/config/usage'),
}
