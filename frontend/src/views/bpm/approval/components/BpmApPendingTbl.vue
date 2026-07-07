<!--
  BpmApPendingTbl.vue - BPM 审批待办任务表
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="tasks" stripe>
      <el-table-column prop="task_name" label="任务名称" min-width="180" fixed />
      <el-table-column prop="process_name" label="流程名称" width="150" />
      <el-table-column prop="start_user_name" label="申请人" width="120" />
      <el-table-column prop="business_key" label="业务单号" width="160" />
      <el-table-column prop="created_at" label="申请时间" width="160" />
      <el-table-column prop="due_date" label="截止时间" width="160">
        <template #default="{ row }">
          <span v-if="row.due_date" :class="{ overdue: isOverdueFmt(row.due_date) }">{{
            row.due_date
          }}</span>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column prop="priority" label="优先级" width="100">
        <template #default="{ row }">
          <el-tag :type="getPriorityTypeFmt(row.priority) as TagType" size="small">{{
            getPriorityTextFmt(row.priority)
          }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="220" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('approve', row as ApprovalTask)"
            >同意</el-button
          >
          <el-button type="danger" link size="small" @click="emit('reject', row as ApprovalTask)"
            >拒绝</el-button
          >
          <el-button type="warning" link size="small" @click="emit('transfer', row as ApprovalTask)"
            >转交</el-button
          >
          <el-button type="info" link size="small" @click="emit('view-chain', row as ApprovalTask)"
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
import type { ApprovalTask } from '@/api/bpm-enhanced'
import { isOverdue, getPriorityType, getPriorityText } from '../composables/bpmApFmts'

// 分页字段类型
interface Pgn {
  page: number
  page_size: number
  total: number
}

/**
 * 审批待办任务表组件
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
  approve: [row: ApprovalTask]
  reject: [row: ApprovalTask]
  transfer: [row: ApprovalTask]
  'view-chain': [row: ApprovalTask]
  'size-change': []
  'current-change': []
}>()

// 透传格式化函数（带 Fmt 后缀以避免与局部命名冲突）
const isOverdueFmt = isOverdue
const getPriorityTypeFmt = getPriorityType
const getPriorityTextFmt = getPriorityText
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
.overdue {
  color: #f56c6c;
  font-weight: 600;
}
</style>
