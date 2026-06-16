/**
 * V2Table 组件类型契约
 * 任务编号: Wave 4 P2-3 V2Table 重做 + 对齐 P2-1 API
 *
 * 设计原则：
 * 1. 对齐 P2-1（test 分支）API 风格：title 字段、可选 width、formatter(row) 签名、renderCell(row) 钩子
 * 2. 保留 P2-3 价值：estimatedRowHeight prop 参数化（页面级行高调优）
 * 3. 单一来源：所有 V2Table 相关类型在此文件导出，避免散落
 */
import type { VNode } from 'vue'

/** 排序列方向 */
export type SortOrder = 'asc' | 'desc'

/** 列定义（ColumnDef） */
export interface ColumnDef {
  /** 数据字段名 */
  key: string
  /** 列标题 */
  title: string
  /** 固定宽度（像素，可选） */
  width?: number
  /** 最小宽度（像素） */
  minWidth?: number
  /** 固定列方向 */
  fixed?: 'left' | 'right'
  /** 是否可排序 */
  sortable?: boolean
  /** 对齐方式 */
  align?: 'left' | 'center' | 'right'
  /**
   * 格式化函数（接收整行 row，返回字符串用于显示）
   * 签名遵循 P2-1：formatter(row) → string
   * 注：与 P2-3 旧签名 formatter(value, row) 不兼容，迁移时需调整
   */
  formatter?: (row: any) => string
  /** 自定义渲染（返回 VNode，优先级高于 formatter） */
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
  /** 总数（用于内置分页，未传则不渲染分页） */
  total?: number
  /** 当前页码（1-based） */
  page?: number
  /** 每页条数 */
  pageSize?: number
  /** 每页条数选项 */
  pageSizes?: number[]
  /** 表格高度（像素或字符串） */
  height?: number | string
  /** 行 key 字段名 */
  rowKey?: string
  /** 空数据文案 */
  emptyText?: string
  /**
   * 估算行高（像素），P2-3 价值保留
   * 4 页面按业务调优：inventory=40, sales=56, production=48, quality=44
   * 默认 48
   */
  estimatedRowHeight?: number
}
