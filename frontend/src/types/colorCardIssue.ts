/**
 * 色卡发放 - 类型定义模块（V15 P0-F11）
 *
 * 设计原则：避免类型重复定义。本模块从 @/api/color-card.ts 重新导出
 * 发放相关类型，同时补充发放业务专用类型（表单状态 / 对话框状态）。
 *
 * 关联文件：
 *   - @/api/color-card.ts（API 客户端 + 核心类型 IssueRecordInfo）
 *   - @/store/colorCardIssue.ts（Pinia store）
 *   - @/composables/useColorCardIssue.ts（业务 composable）
 *   - @/components/color-cards/ColorCardIssueForm.vue（发放表单）
 *   - @/components/color-cards/ColorCardIssueDetail.vue（发放详情操作）
 *   - @/views/color-cards/issues.vue（页面入口，消费上述模块）
 */

// 复用 api/color-card.ts 中已定义的核心类型，避免重复造轮子
export type {
  IssueRecordInfo,
  ColorCardListItem,
  ColorItemInfo,
  ColorCardDetail,
  PagedResponse,
} from '@/api/color-card'

// 复用枚举常量（ISSUE_STATUS / ISSUE_STATUS_COLORS）
export {
  ISSUE_STATUS,
  ISSUE_STATUS_COLORS,
  COLOR_CARD_STATUS,
} from '@/api/color-card'

// ============== 发放业务专用类型 ==============

/** 发放表单状态（与 ColorCardIssueForm.vue 双向绑定） */
export interface IssueFormState {
  color_card_id: number | undefined
  customer_id: number
  issue_qty: number
  expected_return_date: string
  purpose: string
  remark: string
  dye_lot_no: string
}

/** 归还对话框状态 */
export interface ReturnDialogState {
  actual_return_date: string
  remark: string
}

/** 遗失对话框状态 */
export interface LostDialogState {
  compensation_amount: number
  remark: string
}

/** 损坏对话框状态 */
export interface DamagedDialogState {
  compensation_amount: number
  remark: string
}

/** 取消对话框状态 */
export interface CancelDialogState {
  remark: string
}

/** 发放记录查询参数（与后端 ListIssuesQuery 对齐） */
export interface ListIssuesParams {
  color_card_id?: number
  customer_id?: number
  status?: string
  page?: number
  page_size?: number
  from_date?: string
  to_date?: string
}

/** 发放操作类型（用于 ColorCardIssueDetail 组件分发） */
export type IssueAction = 'return' | 'lost' | 'damaged' | 'cancel'
