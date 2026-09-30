import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface CustomerCredit {
  id?: number;
  customer_id?: number;
  customer_name?: string;
  credit_limit?: number;
  used_credit?: number;
  available_credit?: number;
  credit_rating?: string;
  status?: string;
  valid_from?: string;
  valid_to?: string;
  remarks?: string;
  created_at?: string;
  updated_at?: string;
}

export interface CreditRating {
  rating: string;
  credit_limit: number;
  reason?: string;
}

export interface CreditAdjustment {
  amount: number;
  reason: string;
  adjustment_type: 'increase' | 'decrease';
}

/**
 * 信用占用/释放入参：严格对齐后端 `CreditAmountRequest`
 * （backend/src/handlers/customer_credit_handler.rs:82-84，仅一个 `amount` 字段，
 * 且绑定 validate_amount_range → 必须 >0）。后端不接收 business_type/business_id，
 * 多传只会造成"前端以为记了、后端没落库"的契约谎言，故不声明。
 */
export interface CreditOccupation {
  amount: number;
}

export interface CreditEvaluationRequest {
  customer_id: number;
  evaluation_date: string;
}

export interface CustomerCreditQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  status?: string;
  customer_id?: number;
}

/**
 * 后端 `list_credits` 出参是 `ApiResponse<Vec<customer_credit::Model>>`
 * （handlers/customer_credit_handler.rs:87-91），裸数组、无 items 包装；
 * 此前声明成 `{items,total}` 会让按该形状取数的消费方拿到 undefined（恒空不报错）。
 */
export const getCustomerCreditList = (
  params?: CustomerCreditQueryParams
): Promise<ApiResponse<CustomerCredit[]>> => request.get('/crm/customer-credits', { params });

export const getCustomerCredit = (id: number): Promise<ApiResponse<CustomerCredit>> =>
  request.get(`/crm/customer-credits/${id}`);

export const createCustomerCredit = (
  data: Partial<CustomerCredit>
): Promise<ApiResponse<CustomerCredit>> => request.post('/crm/customer-credits', data);

export const updateCustomerCredit = (
  id: number,
  data: Partial<CustomerCredit>
): Promise<ApiResponse<CustomerCredit>> => request.put(`/crm/customer-credits/${id}`, data);

export const deleteCustomerCredit = (id: number): Promise<ApiResponse<void>> =>
  request.delete(`/crm/customer-credits/${id}`);

export const setCreditRating = (
  id: number,
  data: CreditRating
): Promise<ApiResponse<CustomerCredit>> => request.post(`/crm/customer-credits/${id}/rating`, data);

export const occupyCredit = (id: number, data: CreditOccupation): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${id}/occupy`, data);

/**
 * 释放信用额度：后端 `release_credit` 收 `CreditAmountRequest`
 * （backend/src/handlers/customer_credit_handler.rs:192-208，字段只有 `amount`），
 * 服务层按 amount 递减 used_credit，不存在"占用记录 ID"这一入参；
 * 此前声明成 occupation_id 会让请求体缺 `amount` → serde 反序列化失败报参数错误。
 */
export const releaseCredit = (id: number, data: CreditOccupation): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${id}/release`, data);

export const adjustCreditLimit = (
  id: number,
  data: CreditAdjustment
): Promise<ApiResponse<CustomerCredit>> => request.post(`/crm/customer-credits/${id}/adjust`, data);

export const deactivateCredit = (id: number): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${id}/deactivate`);

export const evaluateCustomerCredit = (
  data: CreditEvaluationRequest & { id?: number }
): Promise<ApiResponse<CustomerCredit>> => request.post('/crm/customer-credits/evaluate', data);
