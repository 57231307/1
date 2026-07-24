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
  title: string | null
  phone: string
  email: string | null
  is_primary: boolean
  remarks?: string | null
  created_at?: string
  updated_at?: string
}

/** 联系人创建请求（批次 90b P2-12） */
export interface ContactInput {
  name: string
  title?: string
  phone: string
  email?: string
  is_primary?: boolean
  remarks?: string
}

/** 联系人更新请求（批次 90b P2-12） */
export type ContactUpdate = Partial<ContactInput>

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

// 客户列表（含标签、联系人）
// D14 Batch 5b：原 crmEnhancedApi.getCustomerList 转为风格 B 函数
export const getCustomerList = (params?: QueryParams) =>
  request.get<ApiResponse<PageResult<CustomerWithTags>>>('/crm/customers/enhanced', { params })

// 客户详情
// D14 Batch 5b：原 crmEnhancedApi.getCustomerDetail 转为风格 B 函数
export const getCustomerDetail = (id: number) =>
  request.get<ApiResponse<CustomerWithTags>>(`/crm/customers/enhanced/${id}`)

// 创建客户
// D14 Batch 5b：原 crmEnhancedApi.createCustomer 转为风格 B 函数
export const createCustomer = (data: Partial<CustomerWithTags>) =>
  request.post<ApiResponse<CustomerWithTags>>('/crm/customers/enhanced', data)

// 更新客户
// D14 Batch 5b：原 crmEnhancedApi.updateCustomer 转为风格 B 函数
export const updateCustomer = (id: number, data: Partial<CustomerWithTags>) =>
  request.put<ApiResponse<CustomerWithTags>>(`/crm/customers/enhanced/${id}`, data)

// 删除客户
// D14 Batch 5b：原 crmEnhancedApi.deleteCustomer 转为风格 B 函数
export const deleteCustomer = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/enhanced/${id}`)

// 客户 360 视图
// D14 Batch 5b：原 crmEnhancedApi.getCustomer360 转为风格 B 函数
export const getCustomer360 = (id: number) =>
  request.get<ApiResponse<Customer360>>(`/crm/customers/${id}/360`)

// 标签管理
// D14 Batch 5b：原 crmEnhancedApi.getTags 转为风格 B 函数
export const getCrmTagList = () => request.get<ApiResponse<CustomerTag[]>>('/crm/tags')

// D14 Batch 5b：原 crmEnhancedApi.createTag 转为风格 B 函数
export const createCrmTag = (data: { name: string; color: string; category: string }) =>
  request.post<ApiResponse<CustomerTag>>('/crm/tags', data)

// D14 Batch 5b：原 crmEnhancedApi.deleteTag 转为风格 B 函数
export const deleteCrmTag = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/tags/${id}`)

// D14 Batch 5b：原 crmEnhancedApi.addTagToCustomer 转为风格 B 函数
export const addTagToCustomer = (customerId: number, tagId: number) =>
  request.post<ApiResponse<void>>(`/crm/customers/${customerId}/tags/${tagId}`)

// D14 Batch 5b：原 crmEnhancedApi.removeTagFromCustomer 转为风格 B 函数
export const removeTagFromCustomer = (customerId: number, tagId: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/${customerId}/tags/${tagId}`)

// 公海池
// D14 Batch 5b：原 crmEnhancedApi.getPoolList 转为风格 B 函数
export const getCustomerPoolList = (params?: PoolQueryParams) =>
  request.get<ApiResponse<PageResult<PoolCustomer>>>('/crm/pool', { params })

// D14 Batch 5b：原 crmEnhancedApi.claimFromPool 转为风格 B 函数
export const claimCustomerFromPool = (customerId: number) =>
  request.post<ApiResponse<void>>(`/crm/pool/${customerId}/claim`)

// D14 Batch 5b：原 crmEnhancedApi.batchClaimFromPool 转为风格 B 函数
export const batchClaimCustomersFromPool = (customerIds: number[]) =>
  request.post<ApiResponse<void>>('/crm/pool/batch-claim', { customer_ids: customerIds })

// 回收规则
// D14 Batch 5b：原 crmEnhancedApi.getRecycleRules 转为风格 B 函数
export const getRecycleRuleList = () =>
  request.get<ApiResponse<RecycleRule[]>>('/crm/recycle-rules')

// D14 Batch 5b：原 crmEnhancedApi.createRecycleRule 转为风格 B 函数
export const createRecycleRule = (data: Partial<RecycleRule>) =>
  request.post<ApiResponse<RecycleRule>>('/crm/recycle-rules', data)

// D14 Batch 5b：原 crmEnhancedApi.updateRecycleRule 转为风格 B 函数
export const updateRecycleRule = (id: number, data: Partial<RecycleRule>) =>
  request.put<ApiResponse<RecycleRule>>(`/crm/recycle-rules/${id}`, data)

// D14 Batch 5b：原 crmEnhancedApi.deleteRecycleRule 转为风格 B 函数
export const deleteRecycleRule = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/recycle-rules/${id}`)

// 客户分配
// D14 Batch 5b：原 crmEnhancedApi.assignCustomer 转为风格 B 函数
export const assignCustomer = (data: { customer_ids: number[]; assign_to: number; reason?: string }) =>
  request.post<ApiResponse<void>>('/crm/assignments', data)

// D14 Batch 5b：原 crmEnhancedApi.batchAssign 转为风格 B 函数
export const batchAssignCustomers = (
  data: { assignments: { customer_id: number; assign_to: number }[] }
) => request.post<ApiResponse<void>>('/crm/assignments/batch', data)

// D14 Batch 5b：原 crmEnhancedApi.getAssignmentHistory 转为风格 B 函数
export const getCustomerAssignmentHistory = (params?: AssignmentQueryParams) =>
  request.get<ApiResponse<PageResult<AssignmentRecord>>>('/crm/assignments/history', { params })

// D14 Batch 5b：原 crmEnhancedApi.getSalesUsers 转为风格 B 函数
export const getSalesUserList = () => request.get<ApiResponse<SalesUser[]>>('/crm/sales-users')

// 跟进记录
// D14 Batch 5b：原 crmEnhancedApi.getFollowUps 转为风格 B 函数
export const getFollowUpList = (customerId: number, params?: QueryParams) =>
  request.get<ApiResponse<PageResult<FollowUpRecord>>>(
    `/crm/customers/${customerId}/follow-ups`,
    { params }
  )

// D14 Batch 5b：原 crmEnhancedApi.createFollowUp 转为风格 B 函数
export const createFollowUp = (
  customerId: number,
  data: { type: string; content: string; next_follow_date?: string }
) => request.post<ApiResponse<FollowUpRecord>>(`/crm/customers/${customerId}/follow-ups`, data)

// RFM 模型
// D14 Batch 5b：原 crmEnhancedApi.getRfmScore 转为风格 B 函数
export const getCustomerRfmScore = (customerId: number) =>
  request.get<ApiResponse<RfmScore>>(`/crm/customers/${customerId}/rfm`)

// D14 Batch 5b：原 crmEnhancedApi.getRfmDistribution 转为风格 B 函数
export const getCustomerRfmDistribution = () =>
  request.get<ApiResponse<Record<string, number>>>('/crm/rfm/distribution')

// 释放客户到公海池（P1-5 补齐，与后端 /pool/recycle 对应）
// D14 Batch 5b：原 crmEnhancedApi.recycleToPool 转为风格 B 函数
export const recycleCustomerToPool = (data: { customer_ids: number[]; reason?: string }) =>
  request.post<ApiResponse<void>>('/crm/pool/recycle', data)

// 联系人 CRUD（批次 90b P2-12：替代 detail.vue "新增联系人功能待实现" 占位符）
// D14 Batch 5b：原 crmEnhancedApi.listContacts 转为风格 B 函数
export const getCustomerContactList = (customerId: number) =>
  request.get<ApiResponse<Contact[]>>(`/crm/customers/${customerId}/contacts`)

// D14 Batch 5b：原 crmEnhancedApi.createContact 转为风格 B 函数
export const createCustomerContact = (customerId: number, data: ContactInput) =>
  request.post<ApiResponse<Contact>>(`/crm/customers/${customerId}/contacts`, data)

// D14 Batch 5b：原 crmEnhancedApi.updateContact 转为风格 B 函数
export const updateCustomerContact = (customerId: number, contactId: number, data: ContactUpdate) =>
  request.put<ApiResponse<Contact>>(`/crm/customers/${customerId}/contacts/${contactId}`, data)

// D14 Batch 5b：原 crmEnhancedApi.deleteContact 转为风格 B 函数
export const deleteCustomerContact = (customerId: number, contactId: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/${customerId}/contacts/${contactId}`)
