import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Customer {
  id: number;
  customer_code: string;
  customer_name: string;
  contact_person?: string;
  contact_phone?: string;
  contact_email?: string;
  address?: string;
  city?: string;
  province?: string;
  country?: string;
  postal_code?: string;
  // 后端 models/customer.rs:47 是 NOT NULL `Decimal`，本仓 rust_decimal 未启 serde-float
  // （backend/Cargo.toml:60 只有 features = ["serde"]）→ 序列化线上恒为 JSON 字符串
  // （"1234.56"）。声明成 number 是类型谎言：运行期 .toFixed() / 数值比较 / 算术全部失真。
  // 列 NOT NULL，故不标 ?（标 ? 会诱导消费点写 `?? 0` 把缺键伪造成 0）。
  credit_limit: string;
  payment_terms?: number;
  tax_id?: string;
  bank_name?: string;
  bank_account?: string;
  // 渠道 token，值域 = 后端 constants::customer_type::ALLOWED（retail/wholesale/
  // distributor/manufacturer/other）。列在 DB 侧是 NOT NULL DEFAULT 'other'、实体为
  // 非 Option，响应恒回该键，故此处不标可选（标 ? 会诱导出 `?? ''` 式空值兜底假绿）。
  customer_type: string;
  status: string;
  notes?: string;
  customer_industry?: string;
  main_products?: string;
  // 后端 models/customer.rs:89 是 `Option<Decimal>`（同一未启 serde-float 的 rust_decimal）
  // → 线上是十进制字符串或 null；number 声明会让数值消费点拿到字符串后静默算错。
  annual_purchase: string | null;
  quality_requirement?: string;
  inspection_standard?: string;
  created_at?: string;
  updated_at?: string;
}

export interface CustomerQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  customer_type?: string;
  status?: string;
}

// D14 Batch 5b：原 customerApi.list 与本函数 URL 同为 /crm/customers，判定为重复，移除对象方法，保留本函数
export function getCustomerList(
  params?: CustomerQueryParams
): Promise<ApiResponse<{ items: Customer[]; total: number }>> {
  return request.get('/crm/customers', { params });
}

/**
 * v11 批次 146 P1-4 修复：客户下拉选项统一封装
 *
 * 后端 customer_handler::list_customers（路由 /crm/customers/select）返回
 * ApiResponse<PaginatedResponse<Value>>，data 信封为 { items, total, page, page_size }，
 * 真实列表键为 items。原始分页载荷由 getCustomerSelectPage 暴露，
 * getCustomerSelectList 在其上映射为 {label, value}[]（下拉消费点）。
 *
 * @returns 客户下拉选项数组（label=客户名称, value=客户ID）
 */
export function getCustomerSelectPage(): Promise<
  ApiResponse<{ items: Customer[]; total: number; page: number; page_size: number }>
> {
  return request.get<
    ApiResponse<{ items: Customer[]; total: number; page: number; page_size: number }>
  >('/crm/customers/select');
}

export async function getCustomerSelectList(): Promise<{ label: string; value: number }[]> {
  const res = await getCustomerSelectPage();
  return res.data.items.map(c => ({ label: c.customer_name, value: c.id }));
}

// D14 Batch 5b：原 customerApi.getById 转为风格 B 函数
export const getCustomerById = (id: number) =>
  request.get<ApiResponse<Customer>>(`/crm/customers/${id}`);

// D14 Batch 5b：原 customerApi.create 转为风格 B 函数
/**
 * 创建客户入参 —— 逐键对齐后端 handlers/customer_handler.rs::CreateCustomerRequest（:23-72）。
 * 此前 `extends Partial<Omit<Customer,'credit_limit'>>` 把响应模型 Customer 的只读生成列
 * id/created_at/updated_at 一并冒充为可写入参（后端 DTO 无此三键，serde 静默丢弃=契约漂移）。
 * credit_limit 后端为 Option<String>（:42，JSON number 反序列化即 422），类型只收 string。
 * customer_code 仅创建 DTO 有（:25，Option，未传由服务端按建档规则生成）；更新 DTO 无此键。
 */
export interface CustomerCreatePayload {
  customer_code?: string;
  customer_name: string;
  contact_person?: string;
  contact_phone?: string;
  contact_email?: string;
  address?: string;
  city?: string;
  province?: string;
  postal_code?: string;
  credit_limit?: string;
  payment_terms?: number;
  tax_id?: string;
  bank_name?: string;
  bank_account?: string;
  customer_type?: string;
  country?: string;
  status?: string;
  customer_industry?: string;
  main_products?: string;
  annual_purchase?: number;
  quality_requirement?: string;
  inspection_standard?: string;
  notes?: string;
}

/**
 * 更新客户入参 —— 逐键对齐后端 customer_handler.rs::UpdateCustomerRequest（:93-139）。
 * 全字段 Option（缺键=不改）；DTO 不含 customer_code（编码建档即定，编辑不可改），
 * 也不含 id/created_at/updated_at 只读生成列，故本类型均不接受。
 */
export interface CustomerUpdatePayload {
  customer_name?: string;
  contact_person?: string;
  contact_phone?: string;
  contact_email?: string;
  address?: string;
  city?: string;
  province?: string;
  postal_code?: string;
  credit_limit?: string;
  payment_terms?: number;
  tax_id?: string;
  bank_name?: string;
  bank_account?: string;
  customer_type?: string;
  country?: string;
  status?: string;
  customer_industry?: string;
  main_products?: string;
  annual_purchase?: number;
  quality_requirement?: string;
  inspection_standard?: string;
  notes?: string;
}

export const createCustomer = (data: CustomerCreatePayload) =>
  request.post<ApiResponse<Customer>>('/crm/customers', data);

// D14 Batch 5b：原 customerApi.update 转为风格 B 函数
export const updateCustomer = (id: number, data: CustomerUpdatePayload) =>
  request.put<ApiResponse<Customer>>(`/crm/customers/${id}`, data);

// D14 Batch 5b：原 customerApi.delete 转为风格 B 函数
export const deleteCustomer = (id: number) =>
  request.delete<ApiResponse<null>>(`/crm/customers/${id}`);

/**
 * GET /crm/customers/{id}/credit 的 data 载荷 —— 逐键对齐后端真实出参
 * `handlers/customer_credit_handler.rs:111-133`（`ApiResponse::success(credit)`，
 * credit = `models/customer_credit.rs:10-27` 的 Model 本体序列化，无 DTO 改名）。
 *
 * 此前声明的 `current_balance` / `available` 两个键后端不存在（真实键是 `used_credit` /
 * `available_credit`），属前端自造键：页面"当前占用/可用额度"消费的是 undefined，
 * 与"额度为 0/空"不可区分，是契约谎言而非类型细节，故按实体键集重建而不是把 number 改成 string。
 * credit_limit/used_credit/available_credit 为 NOT NULL Decimal（models/customer_credit.rs:17-19），
 * rust_decimal 未启 serde-float → 线上是 JSON 字符串。
 * 注意：`api/customer-credit.ts::CustomerCredit` 是同一实体的另一处声明（含 credit_rating/
 * valid_from/valid_to/remarks 等后端不存在的键），其收口属另一批契约核查，本类型不复用它。
 */
export interface CustomerCreditInfo {
  id: number;
  customer_id: number;
  customer_name: string | null;
  credit_level: string | null;
  credit_score: number | null;
  credit_limit: string;
  used_credit: string;
  available_credit: string;
  credit_days: number | null;
  last_assessment_date: string | null;
  next_assessment_date: string | null;
  status: string;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

// D14 Batch 5b：原 customerApi.getCreditInfo 转为风格 B 函数
export const getCustomerCreditInfo = (id: number) =>
  request.get<ApiResponse<CustomerCreditInfo>>(`/crm/customers/${id}/credit`);

// D14 Batch 5b：原 customerApi.export 转为风格 B 函数
// V15 P0-S12 + P0-S15 新增（Batch 474）：带水印的 xlsx 导出
// 后端 GET /crm/customers/export 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
// 水印已由后端注入（操作员/IP/时间戳），前端只需下载 Blob
export const exportCustomers = (params?: CustomerQueryParams) =>
  request.get<Blob>('/crm/customers/export', { params, responseType: 'blob' });

// ============== 客户地址簿/CLV/审计日志（Batch 补齐 API 封装）==============

/**
 * 客户收货地址出参（对应后端 models/customer_address.rs::Model 经
 * handlers/customer_address_handler.rs 出参脱敏后的形态）。
 * 脱敏判据与客户域同一真源（services/crm/cust.rs::mask_customer_pii_defaults）：
 * - `contact_phone`：键恒存在；非 admin 会话值是 `utils/field_mask.rs` 的 `mask_phone`
 *   打码结果（形如 138****8888），admin 是原文。两种都是 string，故不标 `?`。
 * - `address`：非 admin 会话**整键不下发**（真源对该键是移除而非打码），admin 下发原文
 *   ⇒ 如实标 optional。消费点禁止 `?? ''` 之类兜底把"无权限查看"吞成"地址为空"：
 *   库内地址为空时后端下发的是空串 string，两者语义不同，必须按键是否存在区分。
 * - 其余列与实体一致（Option 列如实为可选）。
 */
export interface CustomerAddress {
  id: number;
  customer_id: number;
  contact_name: string;
  contact_phone: string;
  province?: string;
  city?: string;
  district?: string;
  address?: string;
  postal_code?: string;
  is_default: boolean;
  remark?: string;
  created_at: string;
  updated_at: string;
}

/** 创建地址请求（对应后端 CreateCustomerAddressDto） */
export interface CustomerAddressInput {
  contact_name: string;
  contact_phone: string;
  province?: string;
  city?: string;
  district?: string;
  address: string;
  postal_code?: string;
  is_default?: boolean;
  remark?: string;
}

/** 更新地址请求（对应后端 UpdateCustomerAddressDto，全字段可选） */
export type CustomerAddressUpdate = Partial<CustomerAddressInput>;

/** 客户全生命周期价值（对应后端 models/customer_lifetime_value.rs::Model） */
export interface CustomerClv {
  id: number;
  customer_id: number;
  total_orders: number;
  total_revenue: number;
  avg_order_value: number;
  first_order_date?: string;
  last_order_date?: string;
  customer_lifespan_days: number;
  purchase_frequency: number;
  clv_score: number;
  /** 客户分层：champion/loyal/potential/at_risk/lost */
  segment?: string;
  calculated_at?: string;
}

/** 客户操作日志（对应后端 models/customer_audit_log.rs::Model） */
export interface CustomerAuditLog {
  id: number;
  customer_id: number;
  /** 操作类型：create/update/delete/view/export */
  operation: string;
  field_name?: string;
  old_value?: string;
  new_value?: string;
  user_id: number;
  user_name: string;
  ip_address?: string;
  user_agent?: string;
  created_at?: string;
}

/** 创建客户操作日志请求（对应后端 crm_handler.rs::CreateAuditLogRequest） */
export interface CustomerAuditLogInput {
  operation: string;
  field_name?: string;
  old_value?: string;
  new_value?: string;
  ip_address?: string;
  user_agent?: string;
}

/**
 * 获取客户收货地址列表（默认地址在前）
 * 后端路由：GET /api/v1/erp/crm/customers/{id}/addresses（routes/crm.rs customers + customer_address_handler）
 * 出参形态见 CustomerAddress：非 admin 会话 contact_phone 为打码值、address 键不下发。
 */
export const getCustomerAddressList = (customerId: number) =>
  request.get<ApiResponse<CustomerAddress[]>>(`/crm/customers/${customerId}/addresses`);

/**
 * 创建客户收货地址（设为默认时后端自动清除其他默认标记）
 * 后端路由：POST /api/v1/erp/crm/customers/{id}/addresses（routes/crm.rs customers + customer_address_handler）
 * 入参提交原文，写响应与读出口同一套脱敏形态（非 admin 会话回的是打码行，不是原文回显）。
 */
export const createCustomerAddress = (customerId: number, data: CustomerAddressInput) =>
  request.post<ApiResponse<CustomerAddress>>(`/crm/customers/${customerId}/addresses`, data);

/**
 * 更新客户收货地址
 * 后端路由：PUT /api/v1/erp/crm/customers/{customer_id}/addresses/{address_id}（routes/crm.rs customers + customer_address_handler）
 * 调用方传 CustomerAddressUpdate：后端逐列 `if let Some` 更新，**缺键=该列不改**。
 * 因此读回来的打码值/缺键列不得原样回填提交（会把掩码写进库、把看不见的列清空），
 * 只提交真正被用户改动的键；写响应同样按读出口脱敏形态下发。
 */
export const updateCustomerAddress = (
  customerId: number,
  addressId: number,
  data: CustomerAddressUpdate
) =>
  request.put<ApiResponse<CustomerAddress>>(
    `/crm/customers/${customerId}/addresses/${addressId}`,
    data
  );

/**
 * 删除客户收货地址
 * 后端路由：DELETE /api/v1/erp/crm/customers/{customer_id}/addresses/{address_id}（routes/crm.rs customers + customer_address_handler）
 */
export const deleteCustomerAddress = (customerId: number, addressId: number) =>
  request.delete<ApiResponse<string>>(`/crm/customers/${customerId}/addresses/${addressId}`);

/**
 * 获取客户 CLV（全生命周期价值；未计算时 data 为 null）
 * 后端路由：GET /api/v1/erp/crm/customers/{id}/clv（routes/crm.rs crm_customer_enhancement_routes）
 */
export const getCustomerClv = (customerId: number) =>
  request.get<ApiResponse<CustomerClv | null>>(`/crm/customers/${customerId}/clv`);

/**
 * 计算并落库客户 CLV（依据全部销售订单）
 * 后端路由：POST /api/v1/erp/crm/customers/{id}/clv/calculate（routes/crm.rs crm_customer_enhancement_routes）
 */
export const calculateCustomerClv = (customerId: number) =>
  request.post<ApiResponse<CustomerClv>>(`/crm/customers/${customerId}/clv/calculate`);

/**
 * 获取客户操作日志列表（可按操作类型过滤）
 * 后端路由：GET /api/v1/erp/crm/customers/{id}/audit-logs（routes/crm.rs crm_customer_enhancement_routes）
 */
export const getCustomerAuditLogs = (customerId: number, params?: { operation?: string }) =>
  request.get<ApiResponse<CustomerAuditLog[]>>(`/crm/customers/${customerId}/audit-logs`, {
    params,
  });

/**
 * 创建客户操作日志（后端返回 data=null）
 * 后端路由：POST /api/v1/erp/crm/customers/{id}/audit-logs（routes/crm.rs crm_customer_enhancement_routes）
 */
export const createCustomerAuditLog = (customerId: number, data: CustomerAuditLogInput) =>
  request.post<ApiResponse<null>>(`/crm/customers/${customerId}/audit-logs`, data);
