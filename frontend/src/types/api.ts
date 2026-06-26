export type { ApiResponse, PaginatedResponse, ErrorResponse, EmptyResponse } from './api-response'

export interface LoginRequest {
  username: string
  password: string
  totp_code?: string
}

export interface LoginResponse {
  token: string
  refresh_token: string
  expires_in: number
  user: UserInfo
  /**
   * FE-P-3 修复（2026-06-26 第二次审计第二优先级）：
   * 后端 LoginResponse 顶层返回 `permissions: Vec<String>`（格式 "{resource}:{action}"），
   * 前端用 `permissions.includes("xxx:yyy")` 判断权限。
   */
  permissions?: string[]
  csrf_token?: string
}

export interface UserInfo {
  id: number
  username: string
  real_name?: string
  email?: string
  phone?: string
  avatar?: string
  role_id?: number
  role_name?: string
  department_id?: number
  department_name?: string
  is_totp_enabled?: boolean
  permissions?: string[]
}

export interface PageResult<T = any> {
  list: T[]
  total: number
  page: number
  page_size: number
}

export interface QueryParams {
  page?: number
  page_size?: number
  keyword?: string
  order_by?: string
  order_dir?: 'asc' | 'desc'
  status?: string | number
  supplier_name?: string
  customer_name?: string
  invoice_no?: string
  voucher_no?: string
  date_range?: string[]
  supplier_id?: number
  customer_id?: number
}

export type StatusType = 'active' | 'inactive' | 'pending' | 'approved' | 'rejected' | 'cancelled'
export type ApprovalStatus = 'pending' | 'approved' | 'rejected' | 'cancelled'
export type PaymentStatus = 'unpaid' | 'partial' | 'paid'
