// 色卡发放类型定义（V15 P0-F11）
//
// 拆分自 frontend/src/api/color-card.ts：将发放相关的类型独立到 types/ 目录
// 与后端 handlers/color_card/issue.rs::IssueRecordInfo 字段对齐
//
// 创建时间：2026-07-18（Batch 477 P0-F11）

// 发放状态字面量类型
export type IssueStatusValue = 'issued' | 'returned' | 'lost' | 'damaged' | 'cancelled'

// 发放记录信息（与后端 IssueRecordInfo 字段对齐）
export interface IssueRecordInfo {
  id: number
  color_card_id: number
  customer_id: number
  issue_qty: number
  issued_by: number
  issued_at: string
  expected_return_date?: string
  actual_return_date?: string
  status: IssueStatusValue
  purpose?: string
  remark?: string
  compensation_amount?: number
  returned_by?: number
  dye_lot_no?: string
  created_at: string
  updated_at: string
}

// 发放记录列表查询参数（与后端 ListIssuesQuery 对齐）
export interface ListIssuesQuery {
  color_card_id?: number
  customer_id?: number
  status?: IssueStatusValue
  page?: number
  page_size?: number
  from_date?: string
  to_date?: string
}

// 创建发放记录 DTO（与后端 IssueColorCardDto 对齐）
export interface CreateIssueDto {
  color_card_id: number
  customer_id: number
  issue_qty?: number
  expected_return_date?: string
  purpose?: string
  remark?: string
  dye_lot_no?: string
}

// 归还 DTO（与后端 ReturnColorCardDto 对齐）
export interface ReturnIssueDto {
  actual_return_date?: string
  remark?: string
}

// 登记遗失 DTO（与后端 MarkLostDto 对齐）
export interface MarkLostDto {
  compensation_amount: number
  remark?: string
}

// 标记损坏 DTO（与后端 MarkDamagedDto 对齐）
export interface MarkDamagedDto {
  compensation_amount?: number
  remark?: string
}

// 取消发放 DTO（与后端 CancelIssueDto 对齐）
export interface CancelIssueDto {
  remark?: string
}

// 色卡库存摘要（V15 P0-F10 库存联动）
export interface ColorCardStockSummary {
  color_card_id: number
  card_no: string
  card_name: string
  stock_quantity: number
  issued_quantity: number
  // 可用库存 = stock_quantity - issued_quantity
  available_quantity: number
}
