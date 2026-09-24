import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

export interface CustomerTag {
  id: number;
  name: string;
  color: string;
  category: string;
  created_at: string;
}

export interface Contact {
  id: number;
  customer_id: number;
  name: string;
  title: string | null;
  phone: string;
  email: string | null;
  is_primary: boolean;
  remarks?: string | null;
  created_at?: string;
  updated_at?: string;
}

/** 联系人创建请求（批次 90b P2-12） */
export interface ContactInput {
  name: string;
  title?: string;
  phone: string;
  email?: string;
  is_primary?: boolean;
  remarks?: string;
}

/** 联系人更新请求（批次 90b P2-12） */
export type ContactUpdate = Partial<ContactInput>;

export interface CustomerWithTags {
  id: number;
  customer_code: string;
  customer_name: string;
  contact_person: string;
  phone: string;
  email: string;
  customer_type: string;
  status: string;
  owner_id: number;
  owner_name: string;
  tags: CustomerTag[];
  contacts: Contact[];
  last_follow_up: string;
  total_orders: number;
  total_amount: number;
  created_at: string;
  updated_at: string;
}

export interface PoolCustomer {
  id: number;
  customer_code: string;
  customer_name: string;
  contact_person: string;
  phone: string;
  email: string;
  customer_type: string;
  source: string;
  days_in_pool: number;
  created_at: string;
}

/** 可分配客户（公海池同构别名，业务上用于分配场景） */
export type AssignableCustomer = PoolCustomer;

export interface RecycleRule {
  id: number;
  name: string;
  /** 未跟进超过 N 天后自动回收到公海 */
  days: number;
  is_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface AssignmentRecord {
  id: number;
  customer_id: number;
  customer_name: string;
  assigned_from: number;
  assigned_from_name: string;
  assigned_to: number;
  assigned_to_name: string;
  assign_type: 'manual' | 'auto' | 'batch';
  reason: string;
  created_at: string;
}

export interface SalesUser {
  id: number;
  name: string;
  department: string;
  customer_count: number;
  active: boolean;
}

export interface RfmScore {
  recency: number;
  frequency: number;
  monetary: number;
  level: 'A' | 'B' | 'C' | 'D' | 'E';
  label: string;
}

export interface FollowUpRecord {
  id: number;
  customer_id: number;
  operator_id: number;
  operator_name: string;
  type: 'phone' | 'meeting' | 'email' | 'wechat' | 'visit';
  content: string;
  next_follow_date: string;
  created_at: string;
}

/** 客户实体（从 360 视图 data.customer 字段取得） */
export interface CustomerEntity {
  id: number;
  customer_code: string;
  customer_name: string;
  contact_person: string;
  phone: string;
  email: string;
  address: string;
  customer_type: string;
  status: string;
  tax_number: string;
  bank_name: string;
  bank_account: string;
  credit_limit: number;
  owner_name: string;
  total_orders: number;
  total_amount: number;
  last_order_date: string;
  created_at: string;
}

/** 360 视图 summary 载荷（包含聚合统计与 RFM 评分） */
export interface Customer360Summary {
  rfm_score: RfmScore;
  [key: string]: unknown;
}

/**
 * GET /crm/customers/{id}/360 的 data 载荷（后端锁定契约）。
 * tags/shipping_addresses 在顶层（非嵌套于 customer）。
 */
export interface Customer360Data {
  customer: CustomerEntity;
  summary: Customer360Summary;
  opportunities: unknown[];
  leads: unknown[];
  recent_orders: unknown[];
  tags: CustomerTag[];
  shipping_addresses: ShippingAddress[];
}

/** @deprecated 使用 Customer360Data 替代（保留向后兼容引用） */
export type Customer360 = Customer360Data;

export interface ShippingAddress {
  id: number;
  customer_id: number;
  contact_name: string;
  contact_phone: string;
  province: string;
  city: string;
  district: string;
  address: string;
  postal_code: string;
  is_default: boolean;
  remark: string;
}

/**
 * GET /crm/pool 查询参数，对齐后端 handlers/crm_pool_handler.rs::PoolQueryParams
 * （无 rename_all，snake_case；全部 Option）。
 */
export interface PoolQueryParams {
  page?: number;
  page_size?: number;
  source?: string;
  industry?: string;
  keyword?: string;
}

/**
 * GET /crm/assignments/history 查询参数，对齐后端
 * services/assignment_history_service.rs::AssignmentHistoryQuery
 * （无 rename_all，snake_case；全部 Option）。
 */
export interface AssignmentQueryParams {
  lead_id?: number;
  user_id?: number;
  action?: string;
  date_from?: string;
  date_to?: string;
  page?: number;
  page_size?: number;
}

/**
 * GET /crm/customers/{id}/follow-ups 查询参数，对齐后端 handlers/crm_handler.rs::FollowUpQuery
 * （仅分页两字段）。
 */
export interface FollowUpListQuery {
  page?: number;
  page_size?: number;
}

/**
 * GET /crm/customers/enhanced 查询参数，对齐后端
 * handlers/crm_customer_handler.rs::CustomerQueryParams（snake_case；全部 Option）。
 */
export interface CustomerListQuery {
  page?: number;
  page_size?: number;
  status?: string;
  keyword?: string;
}

/**
 * GET /crm/assignments/history 真实响应载荷（唯一真相：backend
 * handlers/crm_assignment_handler.rs::list_assignment_history 第 285-288 行，
 * `json!({"items": items, "total": total})`）。注意后端**只返回 items + total**，
 * 无 page/page_size，故不复用 PaginatedResponse；承载列表的键只有 items。
 */
export interface AssignmentHistoryResult {
  items: AssignmentRecord[];
  total: number;
}

/**
 * GET /crm/customers/enhanced 真实响应载荷（唯一真相：handlers/crm_customer_handler.rs::list_customers
 * → services/crm/lead.rs::list_leads 第 155-160 行 `json!({"data": items, "total", "page", "page_size"})`）。
 * 承载列表的键是 `data`（不是 items/list）。元素形状由后端 crm_lead::Model 序列化决定，
 * 与前端 CustomerWithTags 存在字段级差异（见交付报告，本轮仅纠正信封键）。
 */
export interface CustomerPage {
  data: CustomerWithTags[];
  total: number;
  page: number;
  page_size: number;
}

// 客户列表（含标签、联系人）
// D14 Batch 5b：原 crmEnhancedApi.getCustomerList 转为风格 B 函数

// 客户详情
// D14 Batch 5b：原 crmEnhancedApi.getCustomerDetail 转为风格 B 函数

// 创建客户
// D14 Batch 5b：原 crmEnhancedApi.createCustomer 转为风格 B 函数

// 更新客户
// D14 Batch 5b：原 crmEnhancedApi.updateCustomer 转为风格 B 函数

// 删除客户
// D14 Batch 5b：原 crmEnhancedApi.deleteCustomer 转为风格 B 函数

// 客户 360 视图
// D14 Batch 5b：原 crmEnhancedApi.getCustomer360 转为风格 B 函数
export const getCustomer360 = (id: number) =>
  request.get<ApiResponse<Customer360Data>>(`/crm/customers/${id}/360`);

// 标签管理（后端 routes/crm.rs crm_tags()，nest 前缀 /api/v1/erp/crm + /tags）
// D14 Batch 5b：原 crmEnhancedApi.getTags 转为风格 B 函数
export const getCrmTagList = () => request.get<ApiResponse<CustomerTag[]>>('/crm/tags');

// D14 Batch 5b：原 crmEnhancedApi.createTag 转为风格 B 函数
export const createCrmTag = (data: { name: string; color: string; category: string }) =>
  request.post<ApiResponse<CustomerTag>>('/crm/tags', data);

// D14 Batch 5b：原 crmEnhancedApi.deleteTag 转为风格 B 函数
export const deleteCrmTag = (id: number) => request.delete<ApiResponse<void>>(`/crm/tags/${id}`);

// D14 Batch 5b：原 crmEnhancedApi.createTagForCustomer 转为风格 B 函数
export const createTagForCustomer = (customerId: number, tagId: number) =>
  request.post<ApiResponse<void>>(`/crm/customers/${customerId}/tags/${tagId}`);

// D14 Batch 5b：原 crmEnhancedApi.deleteTagFromCustomer 转为风格 B 函数
export const deleteTagFromCustomer = (customerId: number, tagId: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/${customerId}/tags/${tagId}`);

// 公海池
// D14 Batch 5b：原 crmEnhancedApi.getPoolList 转为风格 B 函数
export const getCustomerPoolList = (params?: PoolQueryParams) =>
  request.get<ApiResponse<PaginatedResponse<PoolCustomer>>>('/crm/pool', { params });

// D14 Batch 5b：原 crmEnhancedApi.claimFromPool 转为风格 B 函数
export const claimCustomerFromPool = (customerId: number) =>
  request.post<ApiResponse<void>>(`/crm/pool/${customerId}/claim`);

// D14 Batch 5b：原 crmEnhancedApi.batchClaimFromPool 转为风格 B 函数
export const batchClaimCustomersFromPool = (customerIds: number[]) =>
  request.post<ApiResponse<void>>('/crm/pool/batch-claim', { customer_ids: customerIds });

// 回收规则
// D14 Batch 5b：原 crmEnhancedApi.getRecycleRules 转为风格 B 函数
export const getRecycleRuleList = () =>
  request.get<ApiResponse<RecycleRule[]>>('/crm/recycle-rules');

// D14 Batch 5b：原 crmEnhancedApi.createRecycleRule 转为风格 B 函数
export const createRecycleRule = (data: Partial<RecycleRule>) =>
  request.post<ApiResponse<RecycleRule>>('/crm/recycle-rules', data);

// D14 Batch 5b：原 crmEnhancedApi.updateRecycleRule 转为风格 B 函数
export const updateRecycleRule = (id: number, data: Partial<RecycleRule>) =>
  request.put<ApiResponse<RecycleRule>>(`/crm/recycle-rules/${id}`, data);

// D14 Batch 5b：原 crmEnhancedApi.deleteRecycleRule 转为风格 B 函数
export const deleteRecycleRule = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/recycle-rules/${id}`);

// 客户分配
// D14 Batch 5b：原 crmEnhancedApi.assignCustomer 转为风格 B 函数
export const assignCustomer = (data: {
  customer_ids: number[];
  assign_to: number;
  reason?: string;
}) => request.post<ApiResponse<void>>('/crm/assignments', data);

// D14 Batch 5b：原 crmEnhancedApi.batchAssign 转为风格 B 函数
export const batchAssignCustomers = (data: {
  assignments: { customer_id: number; assign_to: number }[];
}) => request.post<ApiResponse<void>>('/crm/assignments/batch', data);

// D14 Batch 5b：原 crmEnhancedApi.getAssignmentHistory 转为风格 B 函数
export const getCustomerAssignmentHistory = (params?: AssignmentQueryParams) =>
  request.get<ApiResponse<AssignmentHistoryResult>>('/crm/assignments/history', { params });

// D14 Batch 5b：原 crmEnhancedApi.getSalesUsers 转为风格 B 函数
export const getSalesUserList = () => request.get<ApiResponse<SalesUser[]>>('/crm/sales-users');

// 跟进记录
// D14 Batch 5b：原 crmEnhancedApi.getFollowUps 转为风格 B 函数
export const getFollowUpList = (customerId: number, params?: FollowUpListQuery) =>
  request.get<ApiResponse<PaginatedResponse<FollowUpRecord>>>(
    `/crm/customers/${customerId}/follow-ups`,
    {
      params,
    }
  );

// D14 Batch 5b：原 crmEnhancedApi.createFollowUp 转为风格 B 函数
export const createFollowUp = (
  customerId: number,
  data: { type: string; content: string; next_follow_date?: string }
) => request.post<ApiResponse<FollowUpRecord>>(`/crm/customers/${customerId}/follow-ups`, data);

// RFM 模型
// D14 Batch 5b：原 crmEnhancedApi.getRfmScore 转为风格 B 函数
export const getCustomerRfmScore = (customerId: number) =>
  request.get<ApiResponse<RfmScore>>(`/crm/customers/${customerId}/rfm`);

// D14 Batch 5b：原 crmEnhancedApi.getRfmDistribution 转为风格 B 函数
export const getCustomerRfmDistribution = () =>
  request.get<ApiResponse<Record<string, number>>>('/crm/rfm/distribution');

// 释放客户到公海池（P1-5 补齐，与后端 /pool/recycle 对应）
// D14 Batch 5b：原 crmEnhancedApi.recycleToPool 转为风格 B 函数
export const recycleCustomerToPool = (data: { customer_ids: number[]; reason?: string }) =>
  request.post<ApiResponse<void>>('/crm/pool/recycle', data);

// 联系人 CRUD（批次 90b P2-12：替代 detail.vue "新增联系人功能待实现" 占位符）
// D14 Batch 5b：原 crmEnhancedApi.listContacts 转为风格 B 函数
export const getCustomerContactList = (customerId: number) =>
  request.get<ApiResponse<Contact[]>>(`/crm/customers/${customerId}/contacts`);

// D14 Batch 5b：原 crmEnhancedApi.createContact 转为风格 B 函数
export const createCustomerContact = (customerId: number, data: ContactInput) =>
  request.post<ApiResponse<Contact>>(`/crm/customers/${customerId}/contacts`, data);

// D14 Batch 5b：原 crmEnhancedApi.updateContact 转为风格 B 函数
export const updateCustomerContact = (customerId: number, contactId: number, data: ContactUpdate) =>
  request.put<ApiResponse<Contact>>(`/crm/customers/${customerId}/contacts/${contactId}`, data);

// D14 Batch 5b：原 crmEnhancedApi.deleteContact 转为风格 B 函数
export const deleteCustomerContact = (customerId: number, contactId: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/${customerId}/contacts/${contactId}`);

// ===== 客户增强 CRUD（端点 /crm/customers/enhanced，与 customer.ts 的 /customers 是不同域）=====

// D14 Batch 5b：原 crmEnhancedApi.getCustomerList 转为风格 B 函数
export const getCustomerList = (params?: CustomerListQuery) =>
  request.get<ApiResponse<CustomerPage>>('/crm/customers/enhanced', { params });

// D14 Batch 5b：原 crmEnhancedApi.createCustomer 转为风格 B 函数
export const createCustomer = (data: Partial<CustomerWithTags>) =>
  request.post<ApiResponse<CustomerWithTags>>('/crm/customers/enhanced', data);

// D14 Batch 5b：原 crmEnhancedApi.updateCustomer 转为风格 B 函数
export const updateCustomer = (id: number, data: Partial<CustomerWithTags>) =>
  request.put<ApiResponse<CustomerWithTags>>(`/crm/customers/enhanced/${id}`, data);

// D14 Batch 5b：原 crmEnhancedApi.deleteCustomer 转为风格 B 函数
export const deleteCustomer = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/enhanced/${id}`);
