import { request } from './request'
import type { ApiResponse, QueryParams, PageResult } from '@/types/api'

export interface CustomerTag {
  id: number
  name: string
  color: string
  category: string
  created_at: string
}

export interface Contact {
  id: number
  customer_id: number
  name: string
  title: string
  phone: string
  email: string
  is_primary: boolean
  created_at: string
}

export interface CustomerWithTags {
  id: number
  customer_code: string
  customer_name: string
  contact_person: string
  phone: string
  email: string
  customer_type: string
  status: string
  owner_id: number
  owner_name: string
  tags: CustomerTag[]
  contacts: Contact[]
  last_follow_up: string
  total_orders: number
  total_amount: number
  created_at: string
  updated_at: string
}

export interface PoolCustomer {
  id: number
  customer_code: string
  customer_name: string
  contact_person: string
  phone: string
  email: string
  customer_type: string
  source: string
  days_in_pool: number
  created_at: string
}

/** 可分配客户（公海池同构别名，业务上用于分配场景） */
export type AssignableCustomer = PoolCustomer

export interface RecycleRule {
  id: number
  name: string
  days_limit: number
  follow_up_required: boolean
  min_follow_up_count: number
  status: 'active' | 'inactive'
  created_at: string
}

export interface AssignmentRecord {
  id: number
  customer_id: number
  customer_name: string
  assigned_from: number
  assigned_from_name: string
  assigned_to: number
  assigned_to_name: string
  assign_type: 'manual' | 'auto' | 'batch'
  reason: string
  created_at: string
}

export interface SalesUser {
  id: number
  name: string
  department: string
  customer_count: number
  active: boolean
}

export interface RfmScore {
  recency: number
  frequency: number
  monetary: number
  level: 'A' | 'B' | 'C' | 'D' | 'E'
  label: string
}

export interface FollowUpRecord {
  id: number
  customer_id: number
  operator_id: number
  operator_name: string
  type: 'phone' | 'meeting' | 'email' | 'wechat' | 'visit'
  content: string
  next_follow_date: string
  created_at: string
}

export interface Customer360 {
  id: number
  customer_code: string
  customer_name: string
  contact_person: string
  phone: string
  email: string
  address: string
  customer_type: string
  status: string
  tax_number: string
  bank_name: string
  bank_account: string
  credit_limit: number
  owner_name: string
  tags: CustomerTag[]
  contacts: Contact[]
  shipping_addresses: ShippingAddress[]
  follow_ups: FollowUpRecord[]
  rfm_score: RfmScore
  total_orders: number
  total_amount: number
  last_order_date: string
  created_at: string
}

export interface ShippingAddress {
  id: number
  customer_id: number
  name: string
  phone: string
  province: string
  city: string
  district: string
  detail: string
  is_default: boolean
}

export interface PoolQueryParams extends QueryParams {
  keyword?: string
  customer_type?: string
  source?: string
  days_min?: number
  days_max?: number
}

export interface AssignmentQueryParams extends QueryParams {
  customer_id?: number
  assigned_to?: number
  assign_type?: string
  date_range?: string[]
}

export const crmEnhancedApi = {
  // 客户列表（含标签、联系人）
  getCustomerList: (params?: QueryParams) =>
    request.get<ApiResponse<PageResult<CustomerWithTags>>>('/crm/customers/enhanced', { params }),

  // 客户详情
  getCustomerDetail: (id: number) =>
    request.get<ApiResponse<CustomerWithTags>>(`/crm/customers/enhanced/${id}`),

  // 创建客户
  createCustomer: (data: Partial<CustomerWithTags>) =>
    request.post<ApiResponse<CustomerWithTags>>('/crm/customers/enhanced', data),

  // 更新客户
  updateCustomer: (id: number, data: Partial<CustomerWithTags>) =>
    request.put<ApiResponse<CustomerWithTags>>(`/crm/customers/enhanced/${id}`, data),

  // 删除客户
  deleteCustomer: (id: number) =>
    request.delete<ApiResponse<void>>(`/crm/customers/enhanced/${id}`),

  // 客户 360 视图
  getCustomer360: (id: number) => request.get<ApiResponse<Customer360>>(`/crm/customers/${id}/360`),

  // 标签管理
  getTags: () => request.get<ApiResponse<CustomerTag[]>>('/crm/tags'),

  createTag: (data: { name: string; color: string; category: string }) =>
    request.post<ApiResponse<CustomerTag>>('/crm/tags', data),

  deleteTag: (id: number) => request.delete<ApiResponse<void>>(`/crm/tags/${id}`),

  addTagToCustomer: (customerId: number, tagId: number) =>
    request.post<ApiResponse<void>>(`/crm/customers/${customerId}/tags/${tagId}`),

  removeTagFromCustomer: (customerId: number, tagId: number) =>
    request.delete<ApiResponse<void>>(`/crm/customers/${customerId}/tags/${tagId}`),

  // 公海池
  getPoolList: (params?: PoolQueryParams) =>
    request.get<ApiResponse<PageResult<PoolCustomer>>>('/crm/pool', { params }),

  claimFromPool: (customerId: number) =>
    request.post<ApiResponse<void>>(`/crm/pool/${customerId}/claim`),

  batchClaimFromPool: (customerIds: number[]) =>
    request.post<ApiResponse<void>>('/crm/pool/batch-claim', { customer_ids: customerIds }),

  // 回收规则
  getRecycleRules: () => request.get<ApiResponse<RecycleRule[]>>('/crm/recycle-rules'),

  createRecycleRule: (data: Partial<RecycleRule>) =>
    request.post<ApiResponse<RecycleRule>>('/crm/recycle-rules', data),

  updateRecycleRule: (id: number, data: Partial<RecycleRule>) =>
    request.put<ApiResponse<RecycleRule>>(`/crm/recycle-rules/${id}`, data),

  deleteRecycleRule: (id: number) => request.delete<ApiResponse<void>>(`/crm/recycle-rules/${id}`),

  // 客户分配
  assignCustomer: (data: { customer_ids: number[]; assign_to: number; reason?: string }) =>
    request.post<ApiResponse<void>>('/crm/assignments', data),

  batchAssign: (data: { assignments: { customer_id: number; assign_to: number }[] }) =>
    request.post<ApiResponse<void>>('/crm/assignments/batch', data),

  getAssignmentHistory: (params?: AssignmentQueryParams) =>
    request.get<ApiResponse<PageResult<AssignmentRecord>>>('/crm/assignments/history', { params }),

  getSalesUsers: () => request.get<ApiResponse<SalesUser[]>>('/crm/sales-users'),

  // 跟进记录
  getFollowUps: (customerId: number, params?: QueryParams) =>
    request.get<ApiResponse<PageResult<FollowUpRecord>>>(
      `/crm/customers/${customerId}/follow-ups`,
      { params }
    ),

  createFollowUp: (
    customerId: number,
    data: { type: string; content: string; next_follow_date?: string }
  ) => request.post<ApiResponse<FollowUpRecord>>(`/crm/customers/${customerId}/follow-ups`, data),

  // RFM 模型
  getRfmScore: (customerId: number) =>
    request.get<ApiResponse<RfmScore>>(`/crm/customers/${customerId}/rfm`),

  getRfmDistribution: () =>
    request.get<ApiResponse<Record<string, number>>>('/crm/rfm/distribution'),

  // 释放客户到公海池（P1-5 补齐，与后端 /pool/recycle 对应）
  recycleToPool: (data: { customer_ids: number[]; reason?: string }) =>
    request.post<ApiResponse<void>>('/crm/pool/recycle', data),
}

export default crmEnhancedApi
