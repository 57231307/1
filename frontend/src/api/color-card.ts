// 色卡仓储管理 API 客户端
// 16 端点封装
// 创建时间: 2026-06-17

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

// 借出状态
export const BORROW_STATUS = {
  borrowed: '借出中',
  returned: '已归还',
  lost: '遗失',
  damaged: '损坏',
} as const

export const BORROW_STATUS_COLORS: Record<string, string> = {
  borrowed: 'warning',
  returned: 'success',
  lost: 'danger',
  damaged: 'danger',
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

export interface BorrowRecordInfo {
  id: number
  color_card_id: number
  color_card_no?: string
  color_card_name?: string
  customer_id: number
  customer_name?: string
  borrowed_by: number
  borrowed_by_name?: string
  borrowed_at: string
  expected_return_at?: string
  actual_return_at?: string
  status: string
  purpose?: string
  notes?: string
  compensation_amount?: number
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
  return request.get<{ data: PagedResponse<ColorCardListItem> }>('/api/v1/erp/color-cards', { params })
}

export function getColorCard(id: number) {
  return request.get<{ data: ColorCardDetail }>(`/api/v1/erp/color-cards/${id}`)
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
  return request.post<{ data: ColorCardListItem }>('/api/v1/erp/color-cards', dto)
}

export function updateColorCard(id: number, dto: Partial<{
  card_name: string
  card_type: string
  season: string
  brand: string
  description: string
  cover_image_url: string
}>) {
  return request.put<{ data: ColorCardListItem }>(`/api/v1/erp/color-cards/${id}`, dto)
}

export function archiveColorCard(id: number, reason?: string) {
  return request.delete<{ data: ColorCardListItem }>(`/api/v1/erp/color-cards/${id}`, {
    data: { reason },
  })
}

// ============== 色号 CRUD ==============

export function listColorItems(cardId: number, params?: { page?: number; page_size?: number }) {
  return request.get<{ data: PagedResponse<ColorItemInfo> }>(
    `/api/v1/erp/color-cards/${cardId}/items`,
    { params },
  )
}

export function createColorItem(cardId: number, dto: Partial<ColorItemInfo>) {
  return request.post<{ data: ColorItemInfo }>(
    `/api/v1/erp/color-cards/${cardId}/items`,
    dto,
  )
}

export function updateColorItem(cardId: number, itemId: number, dto: Partial<ColorItemInfo>) {
  return request.put<{ data: ColorItemInfo }>(
    `/api/v1/erp/color-cards/${cardId}/items/${itemId}`,
    dto,
  )
}

export function deleteColorItem(cardId: number, itemId: number) {
  return request.delete<{ data: null }>(
    `/api/v1/erp/color-cards/${cardId}/items/${itemId}`,
  )
}

export function batchImportItems(cardId: number, items: Partial<ColorItemInfo>[]) {
  return request.post<{ data: { success_count: number; failed_count: number; errors: any[]; total_colors: number } }>(
    `/api/v1/erp/color-cards/${cardId}/items/batch`,
    { items },
  )
}

// ============== 借出管理 ==============

export function borrowColorCard(dto: {
  color_card_id: number
  customer_id: number
  borrowed_by?: number
  expected_return_at?: string
  purpose?: string
  notes?: string
}) {
  return request.post<{ data: BorrowRecordInfo }>('/api/v1/erp/color-cards/borrow', dto)
}

export function returnColorCard(recordId: number, dto: { actual_return_at?: string; notes?: string }) {
  return request.post<{ data: BorrowRecordInfo }>(
    `/api/v1/erp/color-cards/return/${recordId}`,
    dto,
  )
}

export function markLostColorCard(recordId: number, dto: { compensation_amount: number; notes?: string }) {
  return request.post<{ data: BorrowRecordInfo }>(
    `/api/v1/erp/color-cards/lost/${recordId}`,
    dto,
  )
}

export function markDamagedColorCard(recordId: number, dto: { compensation_amount?: number; notes?: string }) {
  return request.post<{ data: BorrowRecordInfo }>(
    `/api/v1/erp/color-cards/damaged/${recordId}`,
    dto,
  )
}

export function listBorrowRecords(params: {
  color_card_id?: number
  customer_id?: number
  status?: string
  page?: number
  page_size?: number
  from_date?: string
  to_date?: string
}) {
  return request.get<{ data: PagedResponse<BorrowRecordInfo> }>(
    '/api/v1/erp/color-cards/borrow-records',
    { params },
  )
}

// ============== 扫码查询 ==============

export function scanColorCode(code: string) {
  return request.get<{ data: any }>(`/api/v1/erp/color-cards/scan/${encodeURIComponent(code)}`)
}

export function exportColorCardUrl(cardId: number) {
  return `/api/v1/erp/color-cards/export/${cardId}`
}
