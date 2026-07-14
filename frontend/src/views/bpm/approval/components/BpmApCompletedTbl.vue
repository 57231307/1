<!--
  BpmApCompletedTbl.vue - BPM 审批已办任务表
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
  迁移：el-table + el-pagination → V2Table 虚拟滚动表格
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

/**
 * 审批已办任务表组件
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
  'view-chain': [row: ApprovalTask]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()

/** 列定义：任务名称 / 流程名称 / 申请人 / 业务单号 / 审批时间 / 审批结果 / 审批意见 / 操作 */
const columns: ColumnDef<ApprovalTask>[] = [
  { key: 'task_name', title: '任务名称', minWidth: 180 },
  { key: 'process_name', title: '流程名称', width: 150 },
  { key: 'start_user_name', title: '申请人', width: 120 },
  { key: 'business_key', title: '业务单号', width: 160 },
  { key: 'approved_at', title: '审批时间', width: 160 },
  {
    key: 'result',
    title: '审批结果',
    width: 100,
    renderCell: (row: ApprovalTask) =>
      h(
        ElTag,
        { type: row.result === 'approved' ? 'success' : 'danger', size: 'small' },
        { default: () => (row.result === 'approved' ? '同意' : '拒绝') }
      ),
  },
  { key: 'comment', title: '审批意见', minWidth: 200 },
  {
    key: '__actions__',
    title: '操作',
    width: 120,
    renderCell: (row: ApprovalTask) =>
      h(
        ElButton,
        { type: 'info', link: true, size: 'small', onClick: () => emit('view-chain', row) },
        { default: () => '审批链' }
      ),
  },
]
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
</style>
