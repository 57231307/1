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

/**
 * POST /crm/customer-credits/{id}/rating 请求体：严格对齐后端
 * handlers/customer_credit_handler.rs:37-49 CreditRatingRequestDto。
 * 该端点 handler（:137-140）无 Path 形参，customer_id 只从请求体读取（i32 非 Option 必填），
 * 路径 {id} 仅为路由占位（routes/crm.rs:109-110）。
 * 此前声明成 {rating, credit_limit, reason}：rating/reason 是后端不认的自创键（serde 丢弃），
 * 真正的评分键 credit_level/credit_score/credit_days/remark 与必填 customer_id 从未提交——
 * 请求必然 400，即便放宽也会把表单采集的评分/账期数据静默丢掉。
 * 注意与创建端点复用同一 DTO（create_credit 亦收 CreditRatingRequestDto，见 :279-287）。
 */
export interface SetCreditRatingInput {
  customer_id: number;
  /** 信用等级（后端校验 ≤20 字符） */
  credit_level?: string;
  credit_score?: number;
  /** 信用额度：后端 Option<Decimal>，None=保持原值、Some(v)=显式设置（含 0） */
  credit_limit?: number;
  credit_days?: number;
  /** 备注（后端校验 ≤500 字符）；后端键名是单数 remark，非实体出参的 remarks */
  remark?: string;
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
  data: SetCreditRatingInput
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
