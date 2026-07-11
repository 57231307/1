<!--
  BpmApCompletedTbl.vue - BPM 审批已办任务表
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="tasks" stripe>
      <el-table-column prop="task_name" label="任务名称" min-width="180" />
      <el-table-column prop="process_name" label="流程名称" width="150" />
      <el-table-column prop="start_user_name" label="申请人" width="120" />
      <el-table-column prop="business_key" label="业务单号" width="160" />
      <el-table-column prop="approved_at" label="审批时间" width="160" />
      <el-table-column prop="result" label="审批结果" width="100">
        <template #default="{ row }">
          <el-tag :type="row.result === 'approved' ? 'success' : 'danger'" size="small">
            {{ row.result === 'approved' ? '同意' : '拒绝' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="comment"
        label="审批意见"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column label="操作" width="120">
        <template #default="{ row }">
          <el-button type="info" link size="small" @click="emit('view-chain', row as ApprovalTask)"
            >审批链</el-button
          >
        </template>
      </el-table-column>
    </el-table>
    <div class="pagination-wrapper">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
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
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
