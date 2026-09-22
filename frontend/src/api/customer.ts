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
  credit_limit?: number;
  payment_terms?: number;
  tax_id?: string;
  bank_name?: string;
  bank_account?: string;
  customer_type?: string;
  status: string;
  notes?: string;
  customer_industry?: string;
  main_products?: string;
  annual_purchase?: number;
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
/** 创建/更新客户入参：后端 CreateCustomerRequest 的 credit_limit 为字符串（显式格式校验），兼容历史 number 传参 */
export interface CustomerCreatePayload extends Partial<Omit<Customer, 'credit_limit'>> {
  credit_limit?: string | number;
}

export const createCustomer = (data: CustomerCreatePayload) =>
  request.post<ApiResponse<Customer>>('/crm/customers', data);

// D14 Batch 5b：原 customerApi.update 转为风格 B 函数
export const updateCustomer = (id: number, data: CustomerCreatePayload) =>
  request.put<ApiResponse<Customer>>(`/crm/customers/${id}`, data);

// D14 Batch 5b：原 customerApi.delete 转为风格 B 函数
export const deleteCustomer = (id: number) =>
  request.delete<ApiResponse<null>>(`/crm/customers/${id}`);

// D14 Batch 5b：原 customerApi.getCreditInfo 转为风格 B 函数
export const getCustomerCreditInfo = (id: number) =>
  request.get<ApiResponse<{ credit_limit: number; current_balance: number; available: number }>>(
    `/crm/customers/${id}/credit`
  );

// D14 Batch 5b：原 customerApi.export 转为风格 B 函数
// V15 P0-S12 + P0-S15 新增（Batch 474）：带水印的 xlsx 导出
// 后端 GET /crm/customers/export 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
// 水印已由后端注入（操作员/IP/时间戳），前端只需下载 Blob
export const exportCustomers = (params?: CustomerQueryParams) =>
  request.get<Blob>('/crm/customers/export', { params, responseType: 'blob' });

// ============== 客户地址簿/CLV/审计日志（Batch 补齐 API 封装）==============

/** 客户收货地址（对应后端 models/customer_address.rs::Model） */
export interface CustomerAddress {
  id: number;
  customer_id: number;
  contact_name: string;
  contact_phone: string;
  province?: string;
  city?: string;
  district?: string;
  address: string;
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
 */
export const getCustomerAddressList = (customerId: number) =>
  request.get<ApiResponse<CustomerAddress[]>>(`/crm/customers/${customerId}/addresses`);

/**
 * 创建客户收货地址（设为默认时后端自动清除其他默认标记）
 * 后端路由：POST /api/v1/erp/crm/customers/{id}/addresses（routes/crm.rs customers + customer_address_handler）
 */
export const createCustomerAddress = (customerId: number, data: CustomerAddressInput) =>
  request.post<ApiResponse<CustomerAddress>>(`/crm/customers/${customerId}/addresses`, data);

/**
 * 更新客户收货地址
 * 后端路由：PUT /api/v1/erp/crm/customers/{customer_id}/addresses/{address_id}（routes/crm.rs customers + customer_address_handler）
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
