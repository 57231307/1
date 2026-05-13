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
}

export interface UserInfo {
  id: number
  username: string
  real_name: string
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

export interface UserPermission {
  id: number
  name: string
  code: string
  type: string
  resource?: string
  action?: string
}

export interface ApiResponse<T = any> {
  code: number
  message: string
  data: T
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
}

export type StatusType = 'active' | 'inactive' | 'pending' | 'approved' | 'rejected' | 'cancelled'
export type ApprovalStatus = 'pending' | 'approved' | 'rejected' | 'cancelled'
export type PaymentStatus = 'unpaid' | 'partial' | 'paid'
