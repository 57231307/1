// 销售报价单 API 模块
// 基础路径：/quotations（由 request baseURL /api/v1/erp 补全）
// 字段名遵循后端 DTO（snake_case）

import { request } from './request'
import type { ApiResponse } from '@/types/api'

/** 报价单状态（后端 DTO 7 种） */
export type QuotationStatus =
  | 'draft'
  | 'pending_approval'
  | 'approved'
  | 'rejected'
  | 'expired'
  | 'converted'
  | 'cancelled'

/** 货币代码（避免与 @/api/currency 的 Currency 接口冲突） */
export type CurrencyCode = 'CNY' | 'USD' | 'EUR'

/** 价格条款（Incoterms 2020） */
export type PriceTerms = 'FOB' | 'CIF' | 'EXW' | 'DDP' | 'DAP'

/** 客户等级 */
export type CustomerLevel = 'VIP' | 'NORMAL'

/** 贸易条款类型 */
export type TermType = 'logistics' | 'payment' | 'sample' | 'inspection'

/** 创建报价单 DTO（与后端 CreateQuotationDto 一致） */
export interface CreateQuotationDto {
  customer_id: number
  sales_user_id: number
  quotation_date: string
  valid_until: string
  currency: CurrencyCode
  exchange_rate: number
  base_currency: string
  price_terms: PriceTerms
  incoterms_version?: string
  incoterm_location?: string
  tax_inclusive: boolean
  tax_rate: number
  moq?: number
  lead_time_days?: number
  customer_level?: CustomerLevel
  notes?: string
  items: CreateQuotationItemDto[]
  terms?: CreateQuotationTermDto[]
}

/** 创建报价单明细 DTO */
export interface CreateQuotationItemDto {
  product_id: number
  color_id?: number
  specification?: string
  unit: string
  quantity: number
  unit_price: number
  unit_price_with_tax: number
  tier_pricing?: any
  discount_rate?: number
  notes?: string
}

/** 创建贸易条款 DTO */
export interface CreateQuotationTermDto {
  term_type: TermType
  term_key: string
  term_value: string
  sequence: number
}

/** 报价单响应 DTO */
export interface QuotationResponseDto {
  id: number
  quotation_no: string
  customer_id: number
  customer_name?: string
  sales_user_id: number
  sales_user_name?: string
  quotation_date: string
  valid_until: string
  currency: string
  exchange_rate: number
  base_currency?: string
  price_terms: string
  incoterms_version?: string
  incoterm_location?: string
  tax_inclusive: boolean
  tax_rate: number
  moq?: number
  lead_time_days?: number
  customer_level?: string
  status: QuotationStatus
  subtotal: number
  tax_amount: number
  total_amount: number
  approved_by?: number
  approved_by_name?: string
  approved_at?: string
  rejection_reason?: string
  converted_sales_order_id?: number
  converted_at?: string
  notes?: string
  items: QuotationItemResponseDto[]
  terms: QuotationTermResponseDto[]
  created_at: string
  updated_at: string
}

/** 报价单明细响应 DTO */
export interface QuotationItemResponseDto {
  id: number
  product_id: number
  product_name?: string
  product_code?: string
  color_id?: number
  color_code?: string
  pantone_code?: string
  cncs_code?: string
  specification?: string
  unit: string
  quantity: number
  unit_price: number
  unit_price_with_tax: number
  amount: number
  amount_with_tax: number
  tier_pricing?: any
  discount_rate?: number
  discount_amount?: number
  notes?: string
  sequence: number
}

/** 贸易条款响应 DTO */
export interface QuotationTermResponseDto {
  id: number
  term_type: TermType
  term_key: string
  term_value: string
  sequence: number
}

/** 列表查询参数 */
export interface QuotationListQuery {
  page?: number
  page_size?: number
  status?: QuotationStatus
  customer_id?: number
}

/** 价格预计算请求 */
export interface CalculatePriceRequest {
  customer_id: number
  customer_level: CustomerLevel
  product_id: number
  color_id?: number
  quantity: number
  currency: CurrencyCode
  quotation_date: string
}

/** 价格预计算响应 */
export interface CalculatePriceResponse {
  unit_price: number
  unit_price_with_tax: number
  tier_breakdown: Array<{
    min_quantity: number
    max_quantity?: number
    unit_price: number
  }>
  discount_applied: number
  final_amount: number
  price_source: 'color_price' | 'product_price' | 'promotion'
}

/** 转销售订单响应 */
export interface ConvertResponse {
  id: number
  order_no: string
  status: string
}

/** 拒绝原因请求体 */
export interface RejectRequest {
  reason: string
}

/** 列表分页响应（后端实际为数组，部分端点返回 list/total） */
export interface ListResponse {
  items: QuotationResponseDto[]
  total: number
}

/**
 * 列出报价单（分页）
 * @param params 查询参数
 */
export function listQuotations(
  params: QuotationListQuery = {}
): Promise<ApiResponse<QuotationResponseDto[]>> {
  // P2 1-11 修复：去掉 as any，使用显式泛型传递类型契约
  return request.get<ApiResponse<QuotationResponseDto[]>>('/quotations', { params })
}

/**
 * 获取报价单详情
 * @param id 报价单 ID
 */
export function getQuotation(id: number): Promise<ApiResponse<QuotationResponseDto>> {
  return request.get<ApiResponse<QuotationResponseDto>>(`/quotations/${id}`)
}

/**
 * 创建报价单（草稿）
 * @param data 创建数据
 */
export function createQuotation(
  data: CreateQuotationDto
): Promise<ApiResponse<QuotationResponseDto>> {
  return request.post<ApiResponse<QuotationResponseDto>>('/quotations', data)
}

/**
 * 更新报价单（仅 draft / rejected 状态）
 * @param id 报价单 ID
 * @param data 更新数据
 */
export function updateQuotation(
  id: number,
  data: CreateQuotationDto
): Promise<ApiResponse<QuotationResponseDto>> {
  return request.put<ApiResponse<QuotationResponseDto>>(`/quotations/${id}`, data)
}

/**
 * 提交审批（按金额阶梯：<10万自批 / 10-50万经理 / >50万总经理）
 * @param id 报价单 ID
 */
export function submitQuotation(id: number): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/submit`)
}

/**
 * 审批通过
 * @param id 报价单 ID
 */
export function approveQuotation(id: number): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/approve`)
}

/**
 * 审批拒绝
 * @param id 报价单 ID
 * @param reason 拒绝原因
 */
export function rejectQuotation(id: number, reason: string): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/reject`, { reason })
}

/**
 * 取消报价单
 * @param id 报价单 ID
 */
export function cancelQuotation(id: number): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/cancel`)
}

/**
 * 转为销售订单
 * @param id 报价单 ID
 */
export function convertQuotation(id: number): Promise<ApiResponse<ConvertResponse>> {
  return request.post<ApiResponse<ConvertResponse>>(`/quotations/${id}/convert`)
}

/**
 * 获取贸易条款
 * @param id 报价单 ID
 */
export function getQuotationTerms(id: number): Promise<ApiResponse<QuotationTermResponseDto[]>> {
  return request.get<ApiResponse<QuotationTermResponseDto[]>>(`/quotations/${id}/terms`)
}

/**
 * 设置贸易条款（覆盖式）
 * @param id 报价单 ID
 * @param terms 条款列表
 */
export function setQuotationTerms(
  id: number,
  terms: CreateQuotationTermDto[]
): Promise<ApiResponse<QuotationTermResponseDto[]>> {
  return request.put<ApiResponse<QuotationTermResponseDto[]>>(`/quotations/${id}/terms`, { terms })
}

/**
 * 列出即将过期的报价单
 */
export function listExpiringQuotations(): Promise<ApiResponse<QuotationResponseDto[]>> {
  return request.get<ApiResponse<QuotationResponseDto[]>>('/quotations/expiring')
}

/**
 * 列出已过期的报价单
 */
export function listExpiredQuotations(): Promise<ApiResponse<QuotationResponseDto[]>> {
  return request.get<ApiResponse<QuotationResponseDto[]>>('/quotations/expired')
}

/**
 * 价格预计算（不保存）
 * @param data 计算上下文
 */
export function calculatePrice(
  data: CalculatePriceRequest
): Promise<ApiResponse<CalculatePriceResponse>> {
  return request.post<ApiResponse<CalculatePriceResponse>>('/quotations/calculate-price', data)
}

/**
 * 获取色号价格
 * @param productColorId 产品色号 ID
 */
// P2 1-11 修复：去掉 as any 和 any 类型，使用 unknown 占位（后端返回结构待定义 DTO）
export function getColorPrices(productColorId: number): Promise<ApiResponse<unknown[]>> {
  return request.get<ApiResponse<unknown[]>>(`/quotations/color-prices/${productColorId}`)
}

/**
 * 设置色号价格
 * @param productColorId 产品色号 ID
 * @param data 价格数据
 */
export function setColorPrice(
  productColorId: number,
  data: unknown
): Promise<ApiResponse<unknown>> {
  return request.post<ApiResponse<unknown>>(`/quotations/color-prices/${productColorId}`, data)
}

/** 状态码 - 状态标签映射 */
export const QUOTATION_STATUS_LABELS: Record<QuotationStatus, string> = {
  draft: '草稿',
  pending_approval: '待审批',
  approved: '已批准',
  rejected: '已拒绝',
  expired: '已过期',
  converted: '已转订单',
  cancelled: '已取消',
}

/** 状态码 - 标签类型映射（Element Plus tag） */
export const QUOTATION_STATUS_TAG_TYPES: Record<QuotationStatus, string> = {
  draft: 'info',
  pending_approval: 'warning',
  approved: 'success',
  rejected: 'danger',
  expired: 'info',
  converted: 'success',
  cancelled: 'info',
}

/** 价格条款中文标签 */
export const PRICE_TERMS_LABELS: Record<PriceTerms, string> = {
  FOB: 'FOB（装运港船上交货）',
  CIF: 'CIF（成本+保险+运费）',
  EXW: 'EXW（工厂交货）',
  DDP: 'DDP（完税后交货）',
  DAP: 'DAP（目的地交货）',
}

/** 贸易条款类型中文标签 */
export const TERM_TYPE_LABELS: Record<TermType, string> = {
  logistics: '物流条款',
  payment: '付款条件',
  sample: '样品条款',
  inspection: '检验条款',
}
