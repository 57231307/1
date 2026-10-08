// 销售报价单 API 模块
// 基础路径：/quotations（由 request baseURL /api/v1/erp 补全）
// 字段名遵循后端 DTO（snake_case）
// 后端 rust_decimal 未启 serde-floats，出参金额/比率序列化为 JSON 十进制字符串；
// 前端对应字段声明为 string，展示/求和/图表入参走 utils/money.ts 归一。
// 提交 DTO 内 Decimal 字段同样按 string 上送，缺值整键省略（严禁伪造 ''/0 冒充未填）。

import { request } from './request';
import type { ApiResponse } from '@/types/api';
import { numberToDecimalWire } from '@/utils/money';

/** 报价单状态（后端 DTO 7 种） */
export type QuotationStatus =
  'draft' | 'pending_approval' | 'approved' | 'rejected' | 'expired' | 'converted' | 'cancelled';

/** 货币代码（避免与 @/api/currency 的 Currency 接口冲突） */
export type CurrencyCode = 'CNY' | 'USD' | 'EUR';

/** 价格条款（Incoterms 2020） */
export type PriceTerms = 'FOB' | 'CIF' | 'EXW' | 'DDP' | 'DAP';

/** 客户等级 */
export type CustomerLevel = 'VIP' | 'NORMAL';

/** 贸易条款类型 */
export type TermType = 'logistics' | 'payment' | 'sample' | 'inspection';

/**
 * 阶梯定价项：后端 tier_pricing 字段为 JSON 数组（Option<serde_json::Value>），
 * 每项描述一个数量区间的单价；对应单元 Rust 类型见 quotation_pricing_service.rs::TierPrice
 * （min_quantity/max_quantity/unit_price 皆 Decimal），出参形态为 JSON 十进制字符串。
 */
export interface TierPricingItem {
  /** 起订数量（含）；Rust TierPrice.min_quantity: Decimal */
  min_quantity: string;
  /** 截止数量（含）；Rust TierPrice.max_quantity: Option<Decimal>；无上限省略键 */
  max_quantity?: string;
  /** 单价（不含税）；Rust TierPrice.unit_price: Decimal */
  unit_price: string;
  /** 单价（含税）；JSON 扩展键，与 unit_price 同 DecimalWire 口径 */
  unit_price_with_tax?: string;
}

/** 创建报价单 DTO（与后端 quotation_create_dto.rs::CreateQuotationDto 一致） */
export interface CreateQuotationDto {
  customer_id: number;
  sales_user_id: number;
  quotation_date: string;
  valid_until: string;
  currency: CurrencyCode;
  /** Rust: Decimal */
  exchange_rate: string;
  base_currency: string;
  price_terms: PriceTerms;
  incoterms_version?: string;
  incoterm_location?: string;
  tax_inclusive: boolean;
  /** Rust: Decimal */
  tax_rate: string;
  /** Rust: Option<Decimal>；缺值省略键 */
  moq?: string;
  /** Rust: Option<i32> */
  lead_time_days?: number;
  customer_level?: CustomerLevel;
  notes?: string;
  items: CreateQuotationItemDto[];
  terms?: CreateQuotationTermDto[];
}

/** 创建报价单明细 DTO（Rust: CreateQuotationItemDto） */
export interface CreateQuotationItemDto {
  product_id: number;
  /** Rust: Option<i64> */
  color_id?: number;
  specification?: string;
  unit: string;
  /** Rust: Decimal */
  quantity: string;
  /** Rust: Decimal */
  unit_price: string;
  /** Rust: Decimal */
  unit_price_with_tax: string;
  tier_pricing?: TierPricingItem[];
  /** Rust: Option<Decimal>；缺值省略键 */
  discount_rate?: string;
  notes?: string;
}

/** 创建贸易条款 DTO（Rust: CreateQuotationTermDto，无 Decimal 字段） */
export interface CreateQuotationTermDto {
  term_type: TermType;
  term_key: string;
  term_value: string;
  /** Rust: i32 */
  sequence: number;
}

/** 报价单响应 DTO（Rust: QuotationResponseDto） */
export interface QuotationResponseDto {
  /** Rust: i64 */
  id: number;
  quotation_no: string;
  /** Rust: i32 */
  customer_id: number;
  /** 后端 QuotationResponseDto 仅 customer_id，需后端 JOIN customers 补 customer_name */
  customer_name: string | null;
  /** Rust: i64 */
  sales_user_id: number;
  /** 后端 QuotationResponseDto 仅 sales_user_id，需后端 JOIN users 补 sales_user_name */
  sales_user_name: string | null;
  quotation_date: string;
  valid_until: string;
  currency: string;
  /** Rust: Decimal */
  exchange_rate: string;
  base_currency?: string;
  price_terms: string;
  incoterms_version?: string;
  incoterm_location?: string;
  tax_inclusive: boolean;
  /** Rust: Decimal */
  tax_rate: string;
  /** Rust: Option<Decimal> */
  moq?: string;
  /** Rust: Option<i32> */
  lead_time_days?: number;
  customer_level?: string;
  status: QuotationStatus;
  /** Rust: Decimal */
  subtotal: string;
  /** Rust: Decimal */
  tax_amount: string;
  /** Rust: Decimal */
  total_amount: string;
  /** Rust: Option<i64> */
  approved_by?: number;
  approved_at?: string;
  /** 后端 QuotationResponseDto.approved_by_name（Option<String>，service.attach_names 按 approved_by 富化 users.real_name；非实体列） */
  approved_by_name?: string | null;
  /** 后端 QuotationResponseDto.approval_reason（Option<String>，落 sales_quotations.approval_reason 列） */
  approval_reason?: string | null;
  /** 后端 QuotationResponseDto.rejection_reason（Option<String>，落 sales_quotations.rejection_reason 列） */
  rejection_reason?: string | null;
  /** Rust: Option<i64> */
  converted_sales_order_id?: number;
  converted_at?: string;
  notes?: string;
  items: QuotationItemResponseDto[];
  terms: QuotationTermResponseDto[];
  created_at: string;
  updated_at: string;
}

/** 报价单明细响应 DTO（Rust: QuotationItemResponseDto） */
export interface QuotationItemResponseDto {
  /** Rust: i64 */
  id: number;
  /** Rust: i64 */
  product_id: number;
  /** 后端 QuotationItemResponseDto 仅 product_id，需后端 JOIN products 补 product_name */
  product_name: string | null;
  /** 后端 QuotationItemResponseDto 仅 product_id，需后端 JOIN products 补 product_code */
  product_code: string | null;
  /** Rust: Option<i64> */
  color_id?: number;
  color_code?: string;
  pantone_code?: string;
  cncs_code?: string;
  specification?: string;
  unit: string;
  /** Rust: Decimal */
  quantity: string;
  /** Rust: Decimal */
  unit_price: string;
  /** Rust: Decimal */
  unit_price_with_tax: string;
  /** Rust: Decimal */
  amount: string;
  /** Rust: Decimal */
  amount_with_tax: string;
  tier_pricing?: TierPricingItem[];
  /** Rust: Option<Decimal> */
  discount_rate?: string;
  /** Rust: Option<Decimal> */
  discount_amount?: string;
  notes?: string;
  /** Rust: i32 */
  sequence: number;
}

/** 贸易条款响应 DTO（Rust: QuotationTermResponseDto，无 Decimal 字段） */
export interface QuotationTermResponseDto {
  /** Rust: i64 */
  id: number;
  term_type: TermType;
  term_key: string;
  term_value: string;
  /** Rust: i32 */
  sequence: number;
}

/** 列表查询参数 */
export interface QuotationListQuery {
  /** Rust: Option<u64> */
  page?: number;
  /** Rust: Option<u64> */
  page_size?: number;
  status?: QuotationStatus;
  /** Rust: Option<i32> */
  customer_id?: number;
}

/** 价格预计算请求（Rust: PricingContext） */
export interface CalculatePriceRequest {
  /** Rust: i64 */
  customer_id: number;
  customer_level: CustomerLevel;
  /** Rust: i64 */
  product_id: number;
  /** Rust: Option<i64> */
  color_id?: number;
  /** Rust: Decimal */
  quantity: string;
  currency: CurrencyCode;
  quotation_date: string;
}

/** 价格预计算响应（Rust: PricingResult / TierPrice） */
export interface CalculatePriceResponse {
  /** Rust: Decimal */
  unit_price: string;
  /** Rust: Decimal */
  unit_price_with_tax: string;
  tier_breakdown: Array<{
    /** Rust: Decimal */
    min_quantity: string;
    /** Rust: Option<Decimal> */
    max_quantity?: string;
    /** Rust: Decimal */
    unit_price: string;
  }>;
  /** Rust: Decimal */
  discount_applied: string;
  /** Rust: Decimal */
  final_amount: string;
  price_source: 'color_price' | 'product_price' | 'promotion';
}

/** 转销售订单响应（Rust: SalesOrderResponse；无 Decimal 字段） */
export interface ConvertResponse {
  /** Rust: i64 */
  id: number;
  order_no: string;
  status: string;
}

/** 拒绝原因请求体 */
export interface RejectRequest {
  reason: string;
}

/** 列表响应信封类型；真实列表键以各端点出参注为准（见 getQuotationList/getColorPrices 函数注） */
export interface ListResponse {
  items: QuotationResponseDto[];
  total: number;
}

/**
 * 列出报价单（分页）
 * @param params 查询参数
 * 后端出参 { list, total, page, page_size }，真实列表键为 list（非裸数组）。
 */
export function getQuotationList(
  params: QuotationListQuery = {}
): Promise<
  ApiResponse<{ list: QuotationResponseDto[]; total: number; page: number; page_size: number }>
> {
  return request.get<
    ApiResponse<{ list: QuotationResponseDto[]; total: number; page: number; page_size: number }>
  >('/quotations', { params });
}

/**
 * 获取报价单详情
 * @param id 报价单 ID
 */
export function getQuotation(id: number): Promise<ApiResponse<QuotationResponseDto>> {
  return request.get<ApiResponse<QuotationResponseDto>>(`/quotations/${id}`);
}

/**
 * 创建报价单（草稿）
 * @param data 创建数据
 */
export function createQuotation(
  data: CreateQuotationDto
): Promise<ApiResponse<QuotationResponseDto>> {
  return request.post<ApiResponse<QuotationResponseDto>>('/quotations', data);
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
  return request.put<ApiResponse<QuotationResponseDto>>(`/quotations/${id}`, data);
}

/**
 * 提交审批（按金额阶梯：<10万自批 / 10-50万经理 / >50万总经理）
 * @param id 报价单 ID
 */
export function submitQuotation(id: number): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/submit`);
}

/**
 * 审批通过
 * @param id 报价单 ID
 * @param approvalReason 审批通过理由（后端必填，落 sales_quotations.approval_reason 列）
 */
export function approveQuotation(id: number, approvalReason: string): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/approve`, {
    approval_reason: approvalReason,
  });
}

/**
 * 审批拒绝
 * @param id 报价单 ID
 * @param reason 拒绝原因（后端必填，落 sales_quotations.rejection_reason 列）
 */
export function rejectQuotation(id: number, reason: string): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/reject`, { reason });
}

/**
 * 取消报价单
 * @param id 报价单 ID
 */
export function cancelQuotation(id: number): Promise<ApiResponse<null>> {
  return request.post<ApiResponse<null>>(`/quotations/${id}/cancel`);
}

/**
 * 转为销售订单
 * @param id 报价单 ID
 */
export function convertQuotation(id: number): Promise<ApiResponse<ConvertResponse>> {
  return request.post<ApiResponse<ConvertResponse>>(`/quotations/${id}/convert`);
}

/**
 * 获取贸易条款
 * @param id 报价单 ID
 */
export function getQuotationTerms(id: number): Promise<ApiResponse<QuotationTermResponseDto[]>> {
  return request.get<ApiResponse<QuotationTermResponseDto[]>>(`/quotations/${id}/terms`);
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
  return request.put<ApiResponse<QuotationTermResponseDto[]>>(`/quotations/${id}/terms`, { terms });
}

/**
 * 列出即将过期的报价单
 */
export function getExpiringQuotationList(): Promise<ApiResponse<QuotationResponseDto[]>> {
  return request.get<ApiResponse<QuotationResponseDto[]>>('/quotations/expiring');
}

/**
 * 列出已过期的报价单
 */
export function getExpiredQuotationList(): Promise<ApiResponse<QuotationResponseDto[]>> {
  return request.get<ApiResponse<QuotationResponseDto[]>>('/quotations/expired');
}

/**
 * 价格预计算（不保存）
 * @param data 计算上下文
 */
export function calculatePrice(
  data: CalculatePriceRequest
): Promise<ApiResponse<CalculatePriceResponse>> {
  return request.post<ApiResponse<CalculatePriceResponse>>('/quotations/calculate-price', data);
}

/**
 * 获取色号价格（分页）
 * @param productColorId 产品色号 ID
 * 后端出参为 PaginatedResponse 信封，列表键 items；行型后端未定型，以 unknown 承载。
 */
export function getColorPrices(
  productColorId: number
): Promise<ApiResponse<{ items: unknown[]; total: number; page: number; page_size: number }>> {
  return request.get<
    ApiResponse<{ items: unknown[]; total: number; page: number; page_size: number }>
  >(`/quotations/color-prices/${productColorId}`);
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
  return request.post<ApiResponse<unknown>>(`/quotations/color-prices/${productColorId}`, data);
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
};

/** 状态码 - 标签类型映射（Element Plus tag） */
export const QUOTATION_STATUS_TAG_TYPES: Record<QuotationStatus, string> = {
  draft: 'info',
  pending_approval: 'warning',
  approved: 'success',
  rejected: 'danger',
  expired: 'info',
  converted: 'success',
  cancelled: 'info',
};

/** 价格条款中文标签 */
export const PRICE_TERMS_LABELS: Record<PriceTerms, string> = {
  FOB: 'FOB（装运港船上交货）',
  CIF: 'CIF（成本+保险+运费）',
  EXW: 'EXW（工厂交货）',
  DDP: 'DDP（完税后交货）',
  DAP: 'DAP（目的地交货）',
};

/** 贸易条款类型中文标签 */
export const TERM_TYPE_LABELS: Record<TermType, string> = {
  logistics: '物流条款',
  payment: '付款条件',
  sample: '样品条款',
  inspection: '检验条款',
};

// ---------------------------------------------------------------------------
// 编辑态（表单控件持数值 number）与提交边界（写线 string）
// 后端 Decimal 未启 serde-floats，提交/回读线格式都是十进制字符串；
// el-input-number 只能绑定数值，因此表单内部保持 number，最终提交前经 toWire 构造器转换。
// 缺值语义：可选字段 undefined ⇒ 省略键（严禁伪造 ''/0 冒充未填）；
// NOT NULL 字段（quantity/unit_price/unit_price_with_tax/tax_rate/exchange_rate）
// 由表单校验拦截，提交前必然有效数值。
// ---------------------------------------------------------------------------

/** 阶梯定价项编辑态 */
export interface TierPricingEditItem {
  min_quantity: number;
  max_quantity?: number;
  unit_price: number;
  unit_price_with_tax?: number;
}

/** 报价明细行编辑态 */
export interface QuotationItemEditForm {
  product_id?: number;
  color_id?: number;
  specification?: string;
  unit: string;
  quantity: number;
  unit_price: number;
  unit_price_with_tax: number;
  tier_pricing?: TierPricingEditItem[];
  discount_rate?: number;
  notes?: string;
}

/** 报价主表编辑态 */
export interface QuotationEditForm {
  customer_id?: number;
  sales_user_id: number;
  quotation_date: string;
  valid_until: string;
  currency: CurrencyCode;
  exchange_rate: number;
  base_currency: string;
  price_terms: PriceTerms;
  incoterms_version?: string;
  incoterm_location?: string;
  tax_inclusive: boolean;
  tax_rate: number;
  moq?: number;
  lead_time_days?: number;
  customer_level?: CustomerLevel;
  notes?: string;
  items: QuotationItemEditForm[];
  terms?: CreateQuotationTermDto[];
}

function tierPricingToWire(items: TierPricingEditItem[]): TierPricingItem[] {
  return items.map(it => ({
    min_quantity: numberToDecimalWire(it.min_quantity) as string,
    ...(it.max_quantity !== undefined
      ? { max_quantity: numberToDecimalWire(it.max_quantity)! }
      : {}),
    unit_price: numberToDecimalWire(it.unit_price) as string,
    ...(it.unit_price_with_tax !== undefined
      ? { unit_price_with_tax: numberToDecimalWire(it.unit_price_with_tax)! }
      : {}),
  }));
}

/** 明细行编辑态 → 后端 CreateQuotationItemDto（写线 string；可选字段缺值省略键） */
export function quotationItemToWire(row: QuotationItemEditForm): CreateQuotationItemDto {
  return {
    product_id: row.product_id as number,
    ...(row.color_id !== undefined ? { color_id: row.color_id } : {}),
    ...(row.specification !== undefined ? { specification: row.specification } : {}),
    unit: row.unit,
    quantity: numberToDecimalWire(row.quantity) as string,
    unit_price: numberToDecimalWire(row.unit_price) as string,
    unit_price_with_tax: numberToDecimalWire(row.unit_price_with_tax) as string,
    ...(row.tier_pricing ? { tier_pricing: tierPricingToWire(row.tier_pricing) } : {}),
    ...(row.discount_rate !== undefined
      ? { discount_rate: numberToDecimalWire(row.discount_rate)! }
      : {}),
    ...(row.notes !== undefined ? { notes: row.notes } : {}),
  };
}

/** 报价主表编辑态 → 后端 CreateQuotationDto（写线 string；可选字段缺值省略键） */
export function quotationToWire(form: QuotationEditForm): CreateQuotationDto {
  return {
    customer_id: form.customer_id as number,
    sales_user_id: form.sales_user_id,
    quotation_date: form.quotation_date,
    valid_until: form.valid_until,
    currency: form.currency,
    exchange_rate: numberToDecimalWire(form.exchange_rate) as string,
    base_currency: form.base_currency,
    price_terms: form.price_terms,
    ...(form.incoterms_version !== undefined ? { incoterms_version: form.incoterms_version } : {}),
    ...(form.incoterm_location !== undefined ? { incoterm_location: form.incoterm_location } : {}),
    tax_inclusive: form.tax_inclusive,
    tax_rate: numberToDecimalWire(form.tax_rate) as string,
    ...(form.moq !== undefined ? { moq: numberToDecimalWire(form.moq)! } : {}),
    ...(form.lead_time_days !== undefined ? { lead_time_days: form.lead_time_days } : {}),
    ...(form.customer_level !== undefined ? { customer_level: form.customer_level } : {}),
    ...(form.notes !== undefined ? { notes: form.notes } : {}),
    items: form.items.map(quotationItemToWire),
    ...(form.terms ? { terms: form.terms } : {}),
  };
}
