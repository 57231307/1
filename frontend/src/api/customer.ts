import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface Customer {
  id: number
  customer_code: string
  customer_name: string
  contact_person?: string
  contact_phone?: string
  contact_email?: string
  address?: string
  city?: string
  province?: string
  country?: string
  postal_code?: string
  credit_limit?: number
  payment_terms?: number
  tax_id?: string
  bank_name?: string
  bank_account?: string
  customer_type?: string
  status: string
  notes?: string
  customer_industry?: string
  main_products?: string
  annual_purchase?: number
  quality_requirement?: string
  inspection_standard?: string
  created_at?: string
  updated_at?: string
}

export interface CustomerQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  customer_type?: string
  status?: string
}

// D14 Batch 5b：原 customerApi.list 与本函数 URL 同为 /crm/customers，判定为重复，移除对象方法，保留本函数
export function getCustomerList(
  params?: CustomerQueryParams
): Promise<ApiResponse<{ list: Customer[]; total: number }>> {
  return request.get('/crm/customers', { params })
}

/**
 * v11 批次 146 P1-4 修复：客户下拉选项统一封装
 *
 * 背景：arReconciliation/enhanced.vue 和 index.vue 此前直接调用 `request.get('/customers/select')`，
 * 绕过 API 层且响应结构处理错误（期望 `{label, value}[]`，后端返回 PaginatedResponse<Customer>）。
 *
 * 修复：统一封装为 `getCustomerSelectList`，内部调用 `/customers/select` 并映射为 `{label, value}[]` 格式。
 *
 * @returns 客户下拉选项数组（label=客户名称, value=客户ID）
 */
export async function getCustomerSelectList(): Promise<{ label: string; value: number }[]> {
  const res = await request.get<ApiResponse<{ list: Customer[]; total: number } | Customer[]>>('/customers/select')
  const data = res?.data
  const list: Customer[] = (Array.isArray(data) ? data : data?.list) ?? []
  return list.map(c => ({ label: c.customer_name, value: c.id }))
}

// D14 Batch 5b：原 customerApi.getById 转为风格 B 函数
export const getCustomerById = (id: number) =>
  request.get<ApiResponse<Customer>>(`/crm/customers/${id}`)

// D14 Batch 5b：原 customerApi.create 转为风格 B 函数
export const createCustomer = (data: Partial<Customer>) =>
  request.post<ApiResponse<Customer>>('/crm/customers', data)

// D14 Batch 5b：原 customerApi.update 转为风格 B 函数
export const updateCustomer = (id: number, data: Partial<Customer>) =>
  request.put<ApiResponse<Customer>>(`/crm/customers/${id}`, data)

// D14 Batch 5b：原 customerApi.delete 转为风格 B 函数
export const deleteCustomer = (id: number) =>
  request.delete<ApiResponse<null>>(`/crm/customers/${id}`)

// D14 Batch 5b：原 customerApi.getCreditInfo 转为风格 B 函数
export const getCustomerCreditInfo = (id: number) =>
  request.get<ApiResponse<{ credit_limit: number; current_balance: number; available: number }>>(
    `/crm/customers/${id}/credit`
  )

// D14 Batch 5b：原 customerApi.export 转为风格 B 函数
// V15 P0-S12 + P0-S15 新增（Batch 474）：带水印的 xlsx 导出
// 后端 GET /crm/customers/export 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
// 水印已由后端注入（操作员/IP/时间戳），前端只需下载 Blob
export const exportCustomers = (params?: CustomerQueryParams) =>
  request.get<Blob>('/crm/customers/export', { params, responseType: 'blob' })
