// 色卡仓储管理 API 客户端
// 16 端点封装
// 创建时间: 2026-06-17
// V15 P0-F07 重构（2026-07-17）：删除 borrow 模式，替换为 issue 发放模式
// 端点路径相对于 baseURL（/api/v1/erp），不要重复添加前缀，否则会产生双重前缀

import { request } from './request'

// ============== 枚举与常量 ==============

// 色卡类型
export const COLOR_CARD_TYPE = {
  PANTONE: 'PANTONE',
  CNCS: 'CNCS',
  CUSTOM: 'CUSTOM',
} as const

export const COLOR_CARD_TYPE_LABELS: Record<string, string> = {
  PANTONE: 'PANTONE',
  CNCS: 'CNCS',
  CUSTOM: '自定义',
}

// 色卡状态
export const COLOR_CARD_STATUS = {
  active: '在用',
  archived: '已归档',
  lost: '遗失',
} as const

export const COLOR_CARD_STATUS_COLORS: Record<string, string> = {
  active: 'success',
  archived: 'info',
  lost: 'danger',
}

// 发放状态（V15 P0-F07：替代原 BORROW_STATUS，borrow 模式已废弃）
export const ISSUE_STATUS = {
  issued: '发放中',
  returned: '已归还',
  lost: '遗失',
  damaged: '损坏',
  cancelled: '已取消',
} as const

export const ISSUE_STATUS_COLORS: Record<string, string> = {
  issued: 'warning',
  returned: 'success',
  lost: 'danger',
  damaged: 'danger',
  cancelled: 'info',
}

// 季节标签
export const SEASON_LABELS: Record<string, string> = {
  '2024SS': '2024 春夏',
  '2024AW': '2024 秋冬',
  '2025SS': '2025 春夏',
  '2025AW': '2025 秋冬',
  '经典': '经典款',
}

// ============== 类型定义 ==============

export interface ColorCardListItem {
  id: number
  card_no: string
  card_name: string
  card_type: string
  season?: string
  brand?: string
  total_colors: number
  status: string
  cover_image_url?: string
  // V15 P0-F10：色卡总库存数量
  stock_quantity: number
  // V15 P0-F10：已发放数量（可用 = stock_quantity - issued_quantity）
  issued_quantity: number
  created_at: string
}

export interface ColorItemInfo {
  id: number
  color_code: string
  color_name: string
  rgb_r: number
  rgb_g: number
  rgb_b: number
  cmyk_c?: number
  cmyk_m?: number
  cmyk_y?: number
  cmyk_k?: number
  lab_l?: number
  lab_a?: number
  lab_b?: number
  pantone_code?: string
  cncs_code?: string
  custom_code?: string
  hex_value: string
  dye_recipe_id?: number
  product_color_price_id?: number
  swatch_image_url?: string
  sequence: number
}

export interface ColorCardDetail extends ColorCardListItem {
  description?: string
  items: ColorItemInfo[]
  updated_at: string
}
// 发放记录信息（V15 P0-F07：替代原 BorrowRecordInfo，borrow 模式已废弃）
// 与后端 handlers/color_card/issue.rs::IssueRecordInfo 字段对齐
export interface IssueRecordInfo {
  id: number
  color_card_id: number
  customer_id: number
  issue_qty: number
  issued_by: number
  issued_at: string
  expected_return_date?: string
  actual_return_date?: string
  status: string
  purpose?: string
  remark?: string
  compensation_amount?: number
  returned_by?: number
  dye_lot_no?: string
  created_at: string
  updated_at: string
}

export interface PagedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

// ============== 色卡 CRUD ==============

export function listColorCards(params: {
  page?: number
  page_size?: number
  card_type?: string
  season?: string
  status?: string
  keyword?: string
}) {
  return request.get<{ data: PagedResponse<ColorCardListItem> }>('/color-cards', { params })
}

export function getColorCard(id: number) {
  return request.get<{ data: ColorCardDetail }>(`/color-cards/${id}`)
}

export function createColorCard(dto: {
  card_no: string
  card_name: string
  card_type: string
  season?: string
  brand?: string
  description?: string
  cover_image_url?: string
}) {
  return request.post<{ data: ColorCardListItem }>('/color-cards', dto)
}

export function updateColorCard(id: number, dto: Partial<{
  card_name: string
  card_type: string
  season: string
  brand: string
  description: string
  cover_image_url: string
}>) {
  return request.put<{ data: ColorCardListItem }>(`/color-cards/${id}`, dto)
}

export function archiveColorCard(id: number, reason?: string) {
  return request.delete<{ data: ColorCardListItem }>(`/color-cards/${id}`, {
    data: { reason },
  })
}

// ============== 色号 CRUD ==============

export function listColorItems(cardId: number, params?: { page?: number; page_size?: number }) {
  return request.get<{ data: PagedResponse<ColorItemInfo> }>(
    `/color-cards/${cardId}/items`,
    { params },
  )
}

export function createColorItem(cardId: number, dto: Partial<ColorItemInfo>) {
  return request.post<{ data: ColorItemInfo }>(
    `/color-cards/${cardId}/items`,
    dto,
  )
}

export function updateColorItem(cardId: number, itemId: number, dto: Partial<ColorItemInfo>) {
  return request.put<{ data: ColorItemInfo }>(
    `/color-cards/${cardId}/items/${itemId}`,
    dto,
  )
}

export function deleteColorItem(cardId: number, itemId: number) {
  return request.delete<{ data: null }>(
    `/color-cards/${cardId}/items/${itemId}`,
  )
}

// 批次 98 P2-D 修复（v5 复审）：原 errors: any[] 改为显式错误项接口
export interface BatchImportError {
  row: number
  message: string
  field?: string
}

export function batchImportItems(cardId: number, items: Partial<ColorItemInfo>[]) {
  return request.post<
    { data: { success_count: number; failed_count: number; errors: BatchImportError[]; total_colors: number } }
  >(
    `/color-cards/${cardId}/items/batch`,
    { items },
  )
}

// ============== 发放管理（V15 P0-F07：替代原借出管理，borrow 模式已废弃）==============

// 创建发放记录 DTO（与后端 handlers/color_card/issue.rs::IssueColorCardDto 对齐）
export function issueColorCard(dto: {
  color_card_id: number
  customer_id: number
  issue_qty?: number
  expected_return_date?: string
  purpose?: string
  remark?: string
  dye_lot_no?: string
}) {
  return request.post<{ data: IssueRecordInfo }>('/color-cards/issues', dto)
}

// 归还色卡（与后端 ReturnColorCardDto 对齐）
export function returnIssue(recordId: number, dto: { actual_return_date?: string; remark?: string }) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/return`,
    dto,
  )
}

// 登记遗失（与后端 MarkLostDto 对齐）
export function markIssueLost(recordId: number, dto: { compensation_amount: number; remark?: string }) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/lost`,
    dto,
  )
}

// 标记损坏（与后端 MarkDamagedDto 对齐）
export function markIssueDamaged(recordId: number, dto: { compensation_amount?: number; remark?: string }) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/damaged`,
    dto,
  )
}

// 取消发放（与后端 CancelIssueDto 对齐）
export function cancelIssue(recordId: number, dto: { remark?: string }) {
  return request.post<{ data: IssueRecordInfo }>(
    `/color-cards/issues/${recordId}/cancel`,
    dto,
  )
}

// 发放记录详情
export function getIssue(recordId: number) {
  return request.get<{ data: IssueRecordInfo }>(`/color-cards/issues/${recordId}`)
}

// 发放记录列表（与后端 ListIssuesQuery 对齐）
export function listIssues(params: {
  color_card_id?: number
  customer_id?: number
  status?: string
  page?: number
  page_size?: number
  from_date?: string
  to_date?: string
}) {
  return request.get<{ data: PagedResponse<IssueRecordInfo> }>(
    '/color-cards/issues',
    { params },
  )
}

// ============== 扫码查询 ==============

// P2-9c 修复（批次 82 v1 复审）：扫码查询返回类型 any → unknown
export function scanColorCode(code: string) {
  return request.get<{ data: unknown }>(`/color-cards/scan/${encodeURIComponent(code)}`)
}

export function exportColorCardUrl(cardId: number) {
  return `/color-cards/export/${cardId}`
}
