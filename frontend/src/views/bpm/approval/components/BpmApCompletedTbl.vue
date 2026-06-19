<!--
  BpmApCompletedTbl.vue - BPM 审批已办任务表
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
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
          <el-button type="info" link size="small" @click="emit('view-chain', row as any)"
            >审批链</el-button
          >
        </template>
      </el-table-column>
    </el-table>
    <div class="pagination-wrapper">
      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.page_size"
        :total="pagination.total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        @size-change="emit('size-change')"
        @current-change="emit('current-change')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { ApprovalTask } from '@/api/bpm-enhanced'

// 分页字段类型
interface Pgn {
  page: number
  page_size: number
  total: number
}

/**
 * 审批已办任务表组件
 */
defineProps<{
  // 任务列表
  tasks: ApprovalTask[]
  // 加载状态
  loading: boolean
  // 分页信息
  pagination: Pgn
}>()

const emit = defineEmits<{
  'view-chain': [row: ApprovalTask]
  'size-change': []
  'current-change': []
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
