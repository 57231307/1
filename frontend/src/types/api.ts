export type { ApiResponse, PaginatedResponse, ErrorResponse, EmptyResponse } from './api-response'

export interface LoginRequest {
  username: string
  password: string
  /**
   * 批次 29 v7 P0-3 修复：TOTP 字段名从 totp_code 改为 totp_token，对齐后端 LoginRequest。
   * 原前端 totp_code 与后端 totp_token 不一致，登录时 2FA 验证码无法正确传递，
   * 导致开启了 TOTP 的用户无法登录。
   */
  totp_token?: string
  /**
   * v11 批次 141：恢复码登录。当用户开启 2FA 但丢失 TOTP 设备时，
   * 可用恢复码替代 totp_token 进行登录（恢复码一次性使用，消耗后从用户列表删除）。
   */
  recovery_code?: string
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

// P2-9d 修复（批次 82 v1 复审）：默认泛型 any → unknown，强制调用方显式指定泛型类型
// v11 批次 161 CI1 修复：添加可选 items 字段，对齐后端 PaginatedResponse 实际返回结构
// （后端用 items: Vec<T>，前端用 list: T[]；useTableApi 运行时 fallback 兼容两者）
export interface PageResult<T = unknown> {
  list: T[]
  /** 后端 PaginatedResponse 使用 items 字段（可选，便于直接消费后端分页响应） */
  items?: T[]
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
