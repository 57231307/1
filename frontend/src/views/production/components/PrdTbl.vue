<!--
  PrdTbl.vue - 生产管理订单表（V2Table 包装）
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）

  P9-3 清理：原文件级 vue/no-mutating-props disable 注释已删除
  （本组件仅读取 props 传给 V2Table 并 emit 事件，无 prop mutation 行为）
-->
<template>
  <V2Table
    :columns="columns"
    :data="data"
    :loading="loading"
    :page="page"
    :page-size="pageSize"
    :total="total"
    :height="600"
    @page-change="(p: number) => emit('page-change', p)"
    @size-change="(s: number) => emit('size-change', s)"
  />
</template>

<script setup lang="ts">
import { h } from 'vue'
import { ElTag, ElButton } from 'element-plus'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import { PRODUCTION_ORDER_STATUS, type ProductionOrder } from '@/api/production'
// P2-17 修复（批次 86 v2 复审）：h() 渲染函数无法使用 v-permission 指令，
// 改为复用 router 守卫的 hasRoutePermission + useUserStore 做权限判断，
// 行为与 v-permission 指令保持一致（无权限则不渲染该按钮）
import { hasRoutePermission } from '@/router'
import { useUserStore } from '@/store/user'

// 状态 el-tag 类型别名（与 element-plus 类型保持一致）
type ElTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'

/**
 * 权限检查辅助函数（与 v-permission 指令行为等价）
 * @param required 所需权限码
 * @returns 当前用户是否持有该权限
 */
const can = (required: string): boolean => {
  const userStore = useUserStore()
  const permissions = userStore.userInfo?.permissions || []
  return hasRoutePermission(required, permissions)
}

/**
 * 生产管理订单表组件
 */
defineProps<{
  // 列表数据
  data: ProductionOrder[]
  // 加载状态
  loading: boolean
  // 当前页
  page: number
  // 每页大小
  pageSize: number
  // 总数
  total: number
}>()

const emit = defineEmits<{
  'page-change': [page: number]
  'size-change': [size: number]
  'view-detail': [row: ProductionOrder]
  'open-edit': [row: ProductionOrder]
  'status-change': [row: ProductionOrder, status: string]
  delete: [row: ProductionOrder]
}>()

/**
 * 列定义
 * - 计划开始/结束：substring(0, 10) 取日期部分
 * - 状态：使用 PRODUCTION_ORDER_STATUS 嵌套映射 {label, type}
 * - 操作列：按 status 条件渲染不同按钮组
 */
const columns: ColumnDef[] = [
  { key: 'order_no', title: '订单编号', width: 160, fixed: 'left' },
  { key: 'product_name', title: '产品名称', minWidth: 160 },
  { key: 'planned_quantity', title: '计划数量', width: 120, align: 'right' },
  { key: 'actual_quantity', title: '实际数量', width: 120, align: 'right' },
  {
    key: 'scheduled_start_date',
    title: '计划开始',
    width: 140,
    formatter: (row: ProductionOrder) =>
      row.scheduled_start_date ? row.scheduled_start_date.substring(0, 10) : '-',
  },
  {
    key: 'scheduled_end_date',
    title: '计划结束',
    width: 140,
    formatter: (row: ProductionOrder) =>
      row.scheduled_end_date ? row.scheduled_end_date.substring(0, 10) : '-',
  },
  {
    key: 'status',
    title: '状态',
    width: 120,
    align: 'center',
    renderCell: (row: ProductionOrder) => {
      const statusConfig =
        PRODUCTION_ORDER_STATUS[row.status as keyof typeof PRODUCTION_ORDER_STATUS]
      const tagType: ElTagType = (statusConfig?.type as ElTagType) || 'info'
      return h(ElTag, { type: tagType }, { default: () => statusConfig?.label || row.status })
    },
  },
  { key: 'priority', title: '优先级', width: 100, align: 'center' },
  {
    key: '__actions__',
    title: '操作',
    width: 280,
    fixed: 'right',
    renderCell: (row: ProductionOrder) => {
      const buttons: ReturnType<typeof h>[] = [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => emit('view-detail', row) },
          { default: () => '查看' }
        ),
      ]
      if (row.status === 'draft') {
        // P2-17 修复（批次 86 v2 复审）：编辑/删除按钮加权限检查
        // （h() 渲染函数无法用 v-permission 指令，改为 can() 条件 push）
        if (can('production_order:update')) {
          buttons.push(
            h(
              ElButton,
              { type: 'success', link: true, size: 'small', onClick: () => emit('open-edit', row) },
              { default: () => '编辑' }
            )
          )
        }
        buttons.push(
          h(
            ElButton,
            {
              type: 'warning',
              link: true,
              size: 'small',
              onClick: () => emit('status-change', row, 'planned'),
            },
            { default: () => '计划' }
          )
        )
        if (can('production_order:delete')) {
          buttons.push(
            h(
              ElButton,
              { type: 'danger', link: true, size: 'small', onClick: () => emit('delete', row) },
              { default: () => '删除' }
            )
          )
        }
      }
      if (row.status === 'planned') {
        buttons.push(
          h(
            ElButton,
            {
              type: 'primary',
              link: true,
              size: 'small',
              onClick: () => emit('status-change', row, 'in_production'),
            },
            { default: () => '开始生产' }
          )
        )
      }
      if (row.status === 'in_production') {
        buttons.push(
          h(
            ElButton,
            {
              type: 'success',
              link: true,
              size: 'small',
              onClick: () => emit('status-change', row, 'completed'),
            },
            { default: () => '完成' }
          )
        )
      }
      return h('div', { class: 'action-cell' }, buttons)
    },
  },
]
</script>
