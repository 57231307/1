<!--
  BpmApPendingTbl.vue - BPM 审批待办任务表
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
import { h } from 'vue'
import { ElButton, ElTag } from 'element-plus'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import type { ApprovalTask } from '@/api/bpm-enhanced'
import { isOverdue, getPriorityType, getPriorityText } from '../composables/bpmApFmts'

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

/** 列定义：任务名称固定左侧，操作列固定右侧 */
const columns: ColumnDef<ApprovalTask>[] = [
  { key: 'task_name', title: '任务名称', width: 180, fixed: 'left' },
  { key: 'process_name', title: '流程名称', width: 150 },
  { key: 'start_user_name', title: '申请人', width: 120 },
  { key: 'business_key', title: '业务单号', width: 160 },
  { key: 'created_at', title: '申请时间', width: 160 },
  {
    key: 'due_date',
    title: '截止时间',
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
    title: '优先级',
    width: 100,
    // 优先级：el-tag 渲染
    renderCell: (row: ApprovalTask) =>
      h(
        ElTag,
        {
          type: getPriorityType(row.priority) as 'success' | 'warning' | 'info' | 'primary' | 'danger',
          size: 'small',
        },
        { default: () => getPriorityText(row.priority) }
      ),
  },
  {
    key: '__actions__',
    title: '操作',
    width: 220,
    fixed: 'right',
    // 操作列：同意 / 拒绝 / 转交 / 审批链
    renderCell: (row: ApprovalTask) =>
      h('div', { class: 'action-cell' }, [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => emit('approve', row) },
          { default: () => '同意' }
        ),
        h(
          ElButton,
          { type: 'danger', link: true, size: 'small', onClick: () => emit('reject', row) },
          { default: () => '拒绝' }
        ),
        h(
          ElButton,
          { type: 'warning', link: true, size: 'small', onClick: () => emit('transfer', row) },
          { default: () => '转交' }
        ),
        h(
          ElButton,
          { type: 'info', link: true, size: 'small', onClick: () => emit('view-chain', row) },
          { default: () => '审批链' }
        ),
      ]),
  },
]
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
