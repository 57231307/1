import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

export interface CustomerTag {
  id: number;
  name: string;
  color: string;
  /** 后端 customer_tag.category 为 Option<String>，未分类时序列化为 null */
  category: string | null;
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

/**
 * 增强客户列表/详情行类型（GET /crm/customers/enhanced）。
 * 后端该端点直接整行序列化 crm_lead 模型（services/crm/lead.rs::list_leads），
 * 且出口只做掩码/删键不做补键，故出参键集恒 ⊆ crm_lead 字段名集：
 * 成交总额/订单数/最后跟进日/内嵌联系人这类聚合键后端无任何出口提供，
 * 不得声明（联系人唯一真实来源是 /crm/customers/{id}/contacts 独立端点）。
 * customer_code/customer_name/contact_person/phone/customer_type/status 为视图列
 * 当前仍在绑定的历史漂移键（取不到值，待与视图重绑同批收口），其余键与
 * crm_lead 字段一一对应；tags 声明为对象数组与后端字符串数组的形状差异由
 * 消费侧（CustomerListTab 的行类型）收敛。
 */
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

/**
 * GET /crm/sales-users 响应项，对齐后端 handlers/missing_handlers.rs::SalesUser
 * （Serialize，无 rename_all，snake_case）。real_name 为 Option<String> 且后端当前恒置
 * None（用户模型无真实姓名写入通道），需要人名展示/取值时用 username 兜底。
 */
export interface SalesUser {
  id: number;
  username: string;
  real_name: string | null;
  email: string | null;
  phone: string | null;
}

/**
 * GET /crm/customers/{id}/rfm 出参 = backend services/crm/mod.rs::RfmScoreDetail
 * （serde 无 rename_all，键即字段名）。R/F/M 三个分项与合成分 score 均为后端 f64
 * ⇒ 线上是 JSON number（不是 Decimal 那种 JSON 字符串），可直接参与数值渲染，
 * 不需要也不允许在前端反推分项或自造档位词表。
 */
export interface RfmScore {
  recency: number;
  frequency: number;
  monetary: number;
  score: number;
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

/**
 * 客户实体（360 视图 data.customer 整行投影）。
 * 键集 = backend/src/models/customer.rs 的 serde snake_case 字段名，
 * 可空列（Option）如实为 `| null`；非 admin 会话经 handler 两层字段门
 * （crm_handler.rs:1166-1178 → crm/cust.rs::mask_customer_pii_defaults）：
 * contact_phone/contact_email 为已掩码字符串（键保留），address 整键移除（故可选）。
 * 后端出参不含 owner_name / total_orders / total_amount / last_order_date
 * （订单聚合在 summary，负责人姓名当前无任何出口提供）。
 */
export interface CustomerEntity {
  id: number;
  customer_code: string;
  customer_name: string;
  contact_person: string | null;
  contact_phone: string | null;
  contact_email: string | null;
  address?: string | null;
  city: string | null;
  province: string | null;
  country: string | null;
  postal_code: string | null;
  // rust_decimal 出参（Cargo.toml 未启 serde-float）线上是 JSON 字符串，如实声明；
  // 归一只发生在控件绑定/格式化边界，数据层不伪造类型。
  credit_limit: string;
  payment_terms: number;
  tax_id: string | null;
  bank_name: string | null;
  bank_account: string | null;
  status: string;
  customer_type: string;
  notes: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
  customer_industry: string | null;
  main_products: string | null;
  annual_purchase: string | null;
  quality_requirement: string | null;
  inspection_standard: string | null;
  owner_id: number;
  department_id: number | null;
  owner_assigned_at: string | null;
  special_process: string | null;
  source: string | null;
  pool_recycle_reason: string | null;
}

/**
 * 360 视图 summary 载荷 = backend services/crm/mod.rs::CustomerRelationSummary
 * 的真实序列化键（total_order_amount 为 Option<Decimal> → 线上 JSON 字符串或 null）。
 * 该结构体只有以下 7 个键：RFM 评分不在 360 出参内，其唯一出口是独立端点
 * `/crm/customers/{id}/rfm`（见 getCustomerRfmScore），消费方须直接调用它取分。
 */
export interface Customer360Summary {
  customer_id: number;
  total_leads: number;
  total_opportunities: number;
  total_orders: number;
  total_order_amount: string | null;
  last_interaction_at: string | null;
  follow_up_count: number;
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
/**
 * POST /crm/recycle-rules 请求体，对齐后端 services/crm/recycle_rule.rs::CreateRecycleRulePayload
 * （name/days 非 Option 必填且经 #[validate]（name 1-100、days 1-365）+ handler 调用 payload.validate()；
 * is_enabled 为 Option，缺省时后端默认 true）。
 */
export interface CreateRecycleRuleInput {
  name: string;
  days: number;
  is_enabled?: boolean;
}

export const createRecycleRule = (data: CreateRecycleRuleInput) =>
  request.post<ApiResponse<RecycleRule>>('/crm/recycle-rules', data);

// D14 Batch 5b：原 crmEnhancedApi.updateRecycleRule 转为风格 B 函数
/**
 * PUT /crm/recycle-rules/{id} 请求体，对齐后端 recycle_rule.rs::UpdateRecycleRulePayload
 * （三字段均 Option 部分更新；id 走路径，不入 body）。
 */
export interface UpdateRecycleRuleInput {
  name?: string;
  days?: number;
  is_enabled?: boolean;
}

export const updateRecycleRule = (id: number, data: UpdateRecycleRuleInput) =>
  request.put<ApiResponse<RecycleRule>>(`/crm/recycle-rules/${id}`, data);

// D14 Batch 5b：原 crmEnhancedApi.deleteRecycleRule 转为风格 B 函数
export const deleteRecycleRule = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/recycle-rules/${id}`);

// 客户分配
/**
 * POST /crm/assignments 请求体，对齐后端 handlers/crm_assignment_handler.rs::AssignCustomerRequest
 * （Deserialize，无 rename_all；lead_id/assignee_id/assignee_name 非 Option 必填，notes 为 Option）。
 * 分配对象是线索（crm_lead）——公海池/可分配客户列表行的 id 即 lead id。
 * assignee_name 后端必填且无校验来源，前端由所选销售用户 real_name || username 真实推导。
 */
export interface AssignCustomerInput {
  lead_id: number;
  assignee_id: number;
  assignee_name: string;
  notes?: string;
}

// D14 Batch 5b：原 crmEnhancedApi.assignCustomer 转为风格 B 函数
export const assignCustomer = (data: AssignCustomerInput) =>
  request.post<ApiResponse<void>>('/crm/assignments', data);

/**
 * POST /crm/assignments/batch 请求体，对齐后端 crm_assignment_handler.rs::BatchAssignRequest：
 * 批量 = 多条线索分配给同一负责人（lead_ids 数组 + 单 assignee），并非逐条自定义负责人；
 * lead_ids/assignee_id/assignee_name 非 Option 必填，notes 为 Option。
 */
export interface BatchAssignCustomersInput {
  lead_ids: number[];
  assignee_id: number;
  assignee_name: string;
  notes?: string;
}

// D14 Batch 5b：原 crmEnhancedApi.batchAssign 转为风格 B 函数
export const batchAssignCustomers = (data: BatchAssignCustomersInput) =>
  request.post<ApiResponse<void>>('/crm/assignments/batch', data);

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

/**
 * 商机跟进记录创建请求，对齐后端 services/crm/opp.rs::CreateOpportunityFollowUpRequest
 * （该结构体无 #[serde(rename_all)]，按原样 snake_case；follow_up_type/content 必填，
 *  follow_up_time/next_follow_up_date 为 Option）。客户跟进的 {type/next_follow_date} 键集与之不同，
 *  不可复用 createFollowUp。
 */
export interface OpportunityFollowUpInput {
  follow_up_type: string;
  content: string;
  next_follow_up_date?: string;
}

/**
 * 商机跟进记录，对齐后端 models/opportunity_follow_up.rs::Model（handler 经 to_value 序列化为
 * snake_case，Option 列输出 null）。与客户实体跟进 FollowUpRecord 是不同表/不同端点。
 */
export interface OpportunityFollowUpRecord {
  id: number;
  opportunity_id: number;
  follow_up_type: string;
  content: string;
  follow_up_time: string;
  next_follow_up_date: string | null;
  user_id: number;
  user_name: string;
  created_at: string | null;
}

// 商机跟进记录：POST /crm/opportunities/{id}/follow-ups
// 后端 routes/crm.rs:496 create_opportunity_follow_up —— 与客户 /crm/customers/{id}/follow-ups
// 是两个不同域端点，商机跟进必须走此端点，不能把 opportunityId 传给 createFollowUp。
export const createOpportunityFollowUp = (opportunityId: number, data: OpportunityFollowUpInput) =>
  request.post<ApiResponse<OpportunityFollowUpRecord>>(
    `/crm/opportunities/${opportunityId}/follow-ups`,
    data
  );

// RFM 模型
// D14 Batch 5b：原 crmEnhancedApi.getRfmScore 转为风格 B 函数
/**
 * RFM 评分唯一出口（后端 handlers/crm_handler.rs::get_rfm_score）。
 * 客户 360 的 summary 不含 RFM，详情页的 RFM 卡必须走本端点取分。
 */
export const getCustomerRfmScore = (customerId: number) =>
  request.get<ApiResponse<RfmScore>>(`/crm/customers/${customerId}/rfm`);

// D14 Batch 5b：原 crmEnhancedApi.getRfmDistribution 转为风格 B 函数
export const getCustomerRfmDistribution = () =>
  request.get<ApiResponse<Record<string, number>>>('/crm/rfm/distribution');

// 释放客户到公海池（P1-5 补齐，与后端 /pool/recycle 对应）
/**
 * POST /crm/pool/recycle 请求体，对齐后端 handlers/crm_pool_handler.rs::RecycleRequest：
 * 单条回收（lead_id 非 Option 必填；reason 为 Option，空值省略该键），后端无批量形状。
 */
export interface RecycleCustomerInput {
  lead_id: number;
  reason?: string;
}

// D14 Batch 5b：原 crmEnhancedApi.recycleToPool 转为风格 B 函数
export const recycleCustomerToPool = (data: RecycleCustomerInput) =>
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
/**
 * 注意：后端 POST /crm/customers/enhanced 当前以线索域 DTO 反序列化
 * （handlers/crm_customer_handler.rs::create_customer 接收 models/dto/crm_dto.rs::CreateLeadRequest，
 * 落库 crm_lead），与本表单采集的客户域字段（customer_code/customer_name/tax_number/credit_limit/
 * bank_name/bank_account/status 等）不同域，前端任何载荷映射都会丢字段或臆造语义——
 * 属后端契约缺陷，已列入后端串行处理清单，前端不改此函数形状。
 */
export const createCustomer = (data: Partial<CustomerWithTags>) =>
  request.post<ApiResponse<CustomerWithTags>>('/crm/customers/enhanced', data);

// D14 Batch 5b：原 crmEnhancedApi.updateCustomer 转为风格 B 函数
/**
 * PUT /crm/customers/enhanced/{id} 请求体，对齐后端
 * crm_customer_handler.rs::UpdateEnhancedCustomerRequest（Deserialize，无 rename_all，
 * 全部字段 Option——客户域 CustomerService.update_customer 落库；id 走路径不入 body，
 * DTO 无 customer_code 键，多发即被 serde 静默丢弃）。
 * 后端字段名是 contact_phone/contact_email（与客户列表实体 phone/email 不同），tax_number
 * 原样透传（后端映射到 customer.tax_id 落库）。
 */
export interface EnhancedCustomerUpdateInput {
  customer_name?: string;
  contact_person?: string;
  contact_phone?: string;
  contact_email?: string;
  address?: string;
  customer_type?: string;
  tax_number?: string;
  credit_limit?: number;
  bank_name?: string;
  bank_account?: string;
  status?: string;
}

export const updateCustomer = (id: number, data: EnhancedCustomerUpdateInput) =>
  request.put<ApiResponse<CustomerWithTags>>(`/crm/customers/enhanced/${id}`, data);

// D14 Batch 5b：原 crmEnhancedApi.deleteCustomer 转为风格 B 函数
export const deleteCustomer = (id: number) =>
  request.delete<ApiResponse<void>>(`/crm/customers/enhanced/${id}`);
