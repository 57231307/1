/**
 * V2Table 组件类型契约
 * 任务编号: Wave 4 P2-1 PR-1
 * 关联 spec: docs/superpowers/specs/2026-06-16-wave4-p2-1-design.md 第四章
 */
import type { VNode } from 'vue'

/** 排序列方向 */
export type SortOrder = 'asc' | 'desc'

/** 列定义 */
export interface ColumnDef {
  /** 数据字段名 */
  key: string
  /** 列标题 */
  title: string
  /** 固定宽度（像素） */
  width?: number
  /** 最小宽度（像素） */
  minWidth?: number
  /** 固定列方向 */
  fixed?: 'left' | 'right'
  /** 是否可排序 */
  sortable?: boolean
  /** 对齐方式 */
  align?: 'left' | 'center' | 'right'
  /** 格式化函数（返回字符串用于显示）*/
  formatter?: (row: any) => string
  /** 自定义渲染（返回 VNode）*/
  renderCell?: (row: any) => VNode
  /** 是否隐藏 */
  hidden?: boolean
}

/** V2Table 组件 Props */
export interface V2TableProps {
  /** 列定义 */
  columns: ColumnDef[]
  /** 表格数据 */
  data: any[]
  /** 加载状态 */
  loading?: boolean
  /** 总数 */
  total?: number
  /** 当前页码 */
  page?: number
  /** 每页条数 */
  pageSize?: number
  /** 每页条数选项 */
  pageSizes?: number[]
  /** 表格高度（像素或字符串）*/
  height?: number | string
  /** 行 key 字段名 */
  rowKey?: string
  /** 空数据文案 */
  emptyText?: string
  /** 页码变化事件 */
  onPageChange?: (page: number) => void
  /** 每页条数变化事件 */
  onSizeChange?: (size: number) => void
  /** 排序变化事件 */
  onSortChange?: (key: string, order: SortOrder) => void
  /** 行点击事件 */
  onRowClick?: (row: any) => void
}
