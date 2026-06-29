export type { ApiResponse, PaginatedResponse, ErrorResponse, EmptyResponse } from './api-response'

export interface LoginRequest {
  username: string
  password: string
  totp_code?: string
}

export interface LoginResponse {
  /**
   * 批次 24 v6 P0-1 修复：移除 token / refresh_token / expires_in 字段。
   * 后端 LoginResponse 仅返回 { csrf_token, user, permissions }，
   * access_token / refresh_token 通过 httpOnly Cookie 写入，前端不可读。
   * 原 token 字段是死代码（store/user.ts 中 if (responseData.token) 永远为 false）。
   */
  user: UserInfo
  /**
   * FE-P-3 修复（2026-06-26 第二次审计第二优先级）：
   * 后端 LoginResponse 顶层返回 `permissions: Vec<String>`（格式 "{resource}:{action}"），
   * 前端用 `permissions.includes("xxx:yyy")` 判断权限。
   *
   * 批次 22 v5 P0-5 修复：类型改为 readonly，防止前端组件恶意修改权限码数组
   * （如 push 注入 admin:write），运行时配合 Object.freeze 做深度防御。
   *
   * 批次 24 v6 P0-2 修复：UserInfo 内部也包含 permissions 字段，
   * 此顶层字段保留用于兼容 store/user.ts 已有逻辑（顶层优先于 user.permissions）。
   */
  readonly permissions?: readonly string[]
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
  /**
   * 批次 22 v5 P0-5 修复：类型改为 readonly，防止前端组件恶意修改权限码数组
   * （如 push 注入 admin:write），运行时配合 Object.freeze 做深度防御。
   */
  readonly permissions?: readonly string[]
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
