<!--
  BpmApprovalPendingTable.vue - BPM 审批待办任务表
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
  迁移：el-table + el-pagination → V2Table 虚拟滚动表格（内置分页）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <V2Table
      :columns="columns"
      :data="tasks"
      :loading="loading"
      :page="page"
      :page-size="pageSize"
      :page-sizes="[10, 20, 50]"
      :total="total"
      :height="600"
      @page-change="(v: number) => emit('update:page', v)"
      @size-change="(v: number) => emit('update:page-size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { computed, h } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElButton, ElTag } from 'element-plus'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import type { ApprovalTask } from '@/api/bpm-enhanced'
import { isOverdue, getPriorityType } from '../composables/bpmApFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 审批待办任务表组件
 */
defineProps<{
  // 任务列表
  tasks: ApprovalTask[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 当前页
  page: number
  // 每页条数
  pageSize: number
}>()

const emit = defineEmits<{
  approve: [row: ApprovalTask]
  reject: [row: ApprovalTask]
  transfer: [row: ApprovalTask]
  'view-chain': [row: ApprovalTask]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()

// 优先级显示文本（响应式求值，随语言切换更新）
const getPriorityTextFmt = (priority: string) => {
  const map: Record<string, string> = {
    high: t('bpm.priority.high'),
    medium: t('bpm.priority.medium'),
    low: t('bpm.priority.low'),
  }
  return map[priority] || priority
}

/** 列定义：任务名称固定左侧，操作列固定右侧 */
const columns = computed<ColumnDef<ApprovalTask>[]>(() => [
  { key: 'task_name', title: t('bpm.approval.pendingTable.taskName'), width: 180, fixed: 'left' },
  { key: 'process_name', title: t('bpm.approval.pendingTable.processName'), width: 150 },
  { key: 'start_user_name', title: t('bpm.approval.pendingTable.applicant'), width: 120 },
  { key: 'business_key', title: t('bpm.approval.pendingTable.businessKey'), width: 160 },
  { key: 'created_at', title: t('bpm.approval.pendingTable.applyTime'), width: 160 },
  {
    key: 'due_date',
    title: t('bpm.approval.pendingTable.dueDate'),
    width: 160,
    // 截止时间：有值则按超期状态高亮，无值显示 "-"
    renderCell: (row: ApprovalTask) => {
      if (row.due_date) {
        return h('span', { class: { overdue: isOverdue(row.due_date) } }, row.due_date)
      }
      return h('span', '-')
    },
  },
  {
    key: 'priority',
    title: t('bpm.approval.pendingTable.priority'),
    width: 100,
    // 优先级：el-tag 渲染
    renderCell: (row: ApprovalTask) =>
      h(
        ElTag,
        {
          type: getPriorityType(row.priority) as 'success' | 'warning' | 'info' | 'primary' | 'danger',
          size: 'small',
        },
        { default: () => getPriorityTextFmt(row.priority) }
      ),
  },
  {
    key: '__actions__',
    title: t('bpm.approval.pendingTable.operation'),
    width: 220,
    fixed: 'right',
    // 操作列：同意 / 拒绝 / 转交 / 审批链
    renderCell: (row: ApprovalTask) =>
      h('div', { class: 'action-cell' }, [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => emit('approve', row) },
          { default: () => t('bpm.approval.pendingTable.approve') }
        ),
        h(
          ElButton,
          { type: 'danger', link: true, size: 'small', onClick: () => emit('reject', row) },
          { default: () => t('bpm.approval.pendingTable.reject') }
        ),
        h(
          ElButton,
          { type: 'warning', link: true, size: 'small', onClick: () => emit('transfer', row) },
          { default: () => t('bpm.approval.pendingTable.transfer') }
        ),
        h(
          ElButton,
          { type: 'info', link: true, size: 'small', onClick: () => emit('view-chain', row) },
          { default: () => t('bpm.approval.pendingTable.viewChain') }
        ),
      ]),
  },
])
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.action-cell {
  display: flex;
  gap: 4px;
  align-items: center;
}
.overdue {
  color: #f56c6c;
  font-weight: 600;
}
</style>
