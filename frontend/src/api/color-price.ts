// 面料多色号定价扩展 API 客户端
// 16 端点 + 5 枚举 + 4 接口 + 4 辅助函数
// 创建时间: 2026-06-18
// 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md

import { request } from '@/api/request'

// ----------------------------------------------------------------------
// 枚举
// ----------------------------------------------------------------------

export enum Currency {
  CNY = 'CNY',
  USD = 'USD',
  EUR = 'EUR',
}

export enum Season {
  SS = 'SS',         // 春夏
  AW = 'AW',         // 秋冬
  HOLIDAY = 'HOLIDAY', // 节日
}

export enum AdjustmentType {
  PERCENTAGE = 'percentage',
  FIXED = 'fixed',
}

export enum ApprovalStatus {
  PENDING = 'PENDING',
  APPROVED = 'APPROVED',
  REJECTED = 'REJECTED',
}

export enum ChangeType {
  MANUAL = 'manual',
  BATCH = 'batch',
  SEASONAL = 'seasonal',
  CUSTOMER_SPECIFIC = 'customer_specific',
  TIER = 'tier',
}

// ----------------------------------------------------------------------
// 接口
// ----------------------------------------------------------------------

export interface ColorPriceListItem {
  id: number
  product_id: number
  color_id: number
  currency: string
  base_price: string
  effective_from: string
  effective_to: string | null
  customer_level: string | null
  min_quantity: string | null
  max_quantity: string | null
  customer_id: number | null
  season: string | null
  is_active: boolean
  priority: number
  approval_status: string
  created_at: string
  updated_at: string
}

export interface ColorPriceDetail extends ColorPriceListItem {
  notes: string | null
  created_by: number | null
  approved_by: number | null
  approved_at: string | null
  tenant_id: number
}

export interface PriceTier {
  id: number
  product_color_price_id: number
  min_quantity: string
  max_quantity: string | null
  tier_price: string
  customer_level: string | null
  sequence: number
  tenant_id: number
  created_at: string
  updated_at: string
}

export interface CustomerColorPrice {
  id: number
  customer_id: number
  product_id: number
  color_id: number
  special_price: string
  discount_percent: string | null
  currency: string
  valid_from: string
  valid_until: string | null
  notes: string | null
  approved_by: number | null
  approved_at: string | null
  created_at: string
  updated_at: string
}

export interface SeasonalPriceRule {
  id: number
  rule_name: string
  season: string
  product_category_id: number | null
  adjustment_type: string
  adjustment_value: string
  valid_from: string
  valid_until: string | null
  is_active: boolean
  description: string | null
  tenant_id: number
  created_at: string
  updated_at: string
}

export interface PriceHistoryItem {
  id: number
  product_color_price_id: number
  old_price: string
  new_price: string
  currency: string
  change_type: string
  change_reason: string | null
  change_percent: string | null
  quantity: string | null
  operated_by: number
  operated_at: string
  approved_by: number | null
  approved_at: string | null
}

export interface PriceCalcStep {
  step: string
  before: string
  after: string
  rule: string
}

export interface PriceCalcResult {
  base_price: string
  tier_price: string | null
  level_price: string | null
  season_price: string | null
  special_price: string | null
  final_price: string
  currency: string
  applied_rule: string
  breakdown: PriceCalcStep[]
}

// ----------------------------------------------------------------------
// 请求 DTO
// ----------------------------------------------------------------------

export interface CreateColorPriceDto {
  product_id: number
  color_id: number
  currency: string
  base_price: number
  effective_from: string
  effective_to?: string | null
  customer_level?: string | null
  min_quantity?: number | null
  max_quantity?: number | null
  customer_id?: number | null
  season?: string | null
  priority?: number
  notes?: string | null
}

export interface BatchAdjustItem {
  price_id: number
  adjustment_type: 'percentage' | 'fixed'
  adjustment_value: number
}

export interface BatchAdjustDto {
  items: BatchAdjustItem[]
  change_reason?: string
}

export interface ApproveDto {
  decision: 'APPROVED' | 'REJECTED'
  comments?: string
}

export interface CreatePriceTierDto {
  product_color_price_id: number
  min_quantity: number
  max_quantity?: number | null
  tier_price: number
  customer_level?: string | null
  sequence?: number
}

export interface CreateCustomerColorPriceDto {
  customer_id: number
  product_id: number
  color_id: number
  special_price: number
  discount_percent?: number | null
  currency: string
  valid_from: string
  valid_until?: string | null
  notes?: string | null
}

export interface CreateSeasonalRuleDto {
  rule_name: string
  season: 'SS' | 'AW' | 'HOLIDAY'
  product_category_id?: number | null
  adjustment_type: 'percentage' | 'fixed'
  adjustment_value: number
  valid_from: string
  valid_until?: string | null
  description?: string | null
}

export interface ListColorPricesQuery {
  page?: number
  page_size?: number
  product_id?: number
  color_id?: number
  customer_id?: number
  customer_level?: string
  season?: string
  currency?: string
  is_active?: boolean
  approval_status?: string
  keyword?: string
}

export interface PagedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

// ----------------------------------------------------------------------
// 16 个 API 端点
// ----------------------------------------------------------------------

const BASE_URL = '/api/v1/erp/color-prices'

// 1. 列表
export function listColorPrices(params: ListColorPricesQuery) {
  return request.get<PagedResponse<ColorPriceListItem>>(BASE_URL, { params })
}

// 2. 创建
export function createColorPrice(data: CreateColorPriceDto) {
  return request.post<ColorPriceDetail>(BASE_URL, data)
}

// 3. 详情
export function getColorPrice(id: number) {
  return request.get<ColorPriceDetail>(`${BASE_URL}/${id}`)
}

// 4. 更新
export function updateColorPrice(id: number, data: Partial<CreateColorPriceDto>) {
  return request.put<ColorPriceDetail>(`${BASE_URL}/${id}`, data)
}

// 5. 删除
export function deleteColorPrice(id: number) {
  return request.delete<ColorPriceDetail>(`${BASE_URL}/${id}`)
}

// 6. 批量调价
export function batchAdjustColorPrices(data: BatchAdjustDto) {
  return request.post<{
    auto_approved: number[]
    pending_approval: number[]
    total: number
  }>(`${BASE_URL}/batch-adjust`, data)
}

// 7. 审批
export function approveColorPrice(id: number, data: ApproveDto) {
  return request.post<ColorPriceDetail>(`${BASE_URL}/${id}/approve`, data)
}

// 8. 价格历史
export function getColorPriceHistory(id: number) {
  return request.get<PagedResponse<PriceHistoryItem>>(`${BASE_URL}/${id}/history`)
}

// 9. 价格计算
export function calculateColorPrice(params: {
  product_id: number
  color_id: number
  customer_id?: number
  customer_level?: string
  quantity: number
  season?: string
  product_category_id?: number
  currency?: string
  calc_date?: string
}) {
  return request.get<PriceCalcResult>(`${BASE_URL}/calculate`, { params })
}

// 10. 阶梯价列表
export function listTiers(priceId: number) {
  return request.get<{ items: PriceTier[]; total: number }>(`${BASE_URL}/tiers/${priceId}`)
}

// 11. 阶梯价创建
export function createTier(data: CreatePriceTierDto) {
  return request.post<PriceTier>(`${BASE_URL}/tiers/${data.product_color_price_id}`, data)
}

// 12. 阶梯价删除
export function deleteTier(tierId: number) {
  return request.delete<{ deleted: number }>(`${BASE_URL}/tiers/item/${tierId}`)
}

// 13. 客户专属价列表
export function listCustomerSpecialPrices() {
  return request.get<{ items: CustomerColorPrice[]; total: number }>(`${BASE_URL}/customer-special`)
}

// 14. 客户专属价创建
export function createCustomerSpecialPrice(data: CreateCustomerColorPriceDto) {
  return request.post<CustomerColorPrice>(`${BASE_URL}/customer-special`, data)
}

// 15. 季节规则列表
export function listSeasonalRules(params: {
  page?: number
  page_size?: number
  season?: string
  is_active?: boolean
  product_category_id?: number
}) {
  return request.get<{ items: SeasonalPriceRule[]; total: number; page: number; page_size: number }>(
    `${BASE_URL}/seasonal-rules`,
    { params },
  )
}

// 16. 季节规则创建
export function createSeasonalRule(data: CreateSeasonalRuleDto) {
  return request.post<SeasonalPriceRule>(`${BASE_URL}/seasonal-rules`, data)
}

// ----------------------------------------------------------------------
// 辅助函数
// ----------------------------------------------------------------------

/** 格式化价格（带币种符号） */
export function formatPrice(price: string | number, currency: string = 'CNY'): string {
  const num = typeof price === 'string' ? parseFloat(price) : price
  const symbol = currency === 'CNY' ? '¥' : currency === 'USD' ? '$' : '€'
  return `${symbol}${num.toFixed(2)}`
}

/** 获取客户等级标签 */
export function getLevelLabel(level: string | null | undefined): string {
  switch (level) {
    case 'VIP':
      return 'VIP 客户'
    case 'GOLD':
      return '金牌客户'
    case 'SILVER':
      return '银牌客户'
    case 'NORMAL':
      return '普通客户'
    default:
      return '通用'
  }
}

/** 获取客户等级标签颜色 */
export function getLevelColor(level: string | null | undefined): string {
  switch (level) {
    case 'VIP':
      return 'red'
    case 'GOLD':
      return 'orange'
    case 'SILVER':
      return 'gold'
    default:
      return 'blue'
  }
}

/** 获取季节标签 */
export function getSeasonLabel(season: string | null | undefined): string {
  switch (season) {
    case 'SS':
      return '春夏'
    case 'AW':
      return '秋冬'
    case 'HOLIDAY':
      return '节日'
    default:
      return '通用'
  }
}

/** 获取季节颜色 */
export function getSeasonColor(season: string | null | undefined): string {
  switch (season) {
    case 'SS':
      return 'green'
    case 'AW':
      return 'orange'
    case 'HOLIDAY':
      return 'red'
    default:
      return 'default'
  }
}

/** 获取审批状态标签 */
export function getApprovalLabel(status: string): string {
  switch (status) {
    case 'PENDING':
      return '待审批'
    case 'APPROVED':
      return '已通过'
    case 'REJECTED':
      return '已拒绝'
    default:
      return status
  }
}

/** 获取审批状态颜色 */
export function getApprovalColor(status: string): string {
  switch (status) {
    case 'PENDING':
      return 'orange'
    case 'APPROVED':
      return 'green'
    case 'REJECTED':
      return 'red'
    default:
      return 'default'
  }
}
