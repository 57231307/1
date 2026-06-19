<!--
  SuTaskTbl.vue - 系统更新任务表格
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="tasks" stripe>
      <el-table-column prop="task_code" label="任务编号" width="140" />
      <el-table-column prop="from_version" label="原版本" width="100" />
      <el-table-column prop="to_version" label="目标版本" width="100" />
      <el-table-column prop="status" label="状态" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="getTaskStatusType(row.status)" size="small">
            {{ getTaskStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="progress" label="进度" width="150">
        <template #default="{ row }">
          <el-progress
            :percentage="row.progress"
            :status="
              row.status === 'failed'
                ? 'exception'
                : row.status === 'completed'
                  ? 'success'
                  : undefined
            "
          />
        </template>
      </el-table-column>
      <el-table-column
        prop="error_message"
        label="错误信息"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column prop="started_at" label="开始时间" width="160" />
      <el-table-column prop="completed_at" label="完成时间" width="160" />
      <el-table-column label="操作" width="150" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'completed'"
            type="warning"
            link
            size="small"
            @click="emit('rollback', row as UpdateTask)"
            >回滚</el-button
          >
          <el-button
            v-if="
              row.status === 'pending' ||
              row.status === 'downloading' ||
              row.status === 'installing'
            "
            type="danger"
            link
            size="small"
            @click="emit('cancel', row as UpdateTask)"
            >取消</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="query.page"
        :page-size="query.page_size"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('refresh')"
        @current-change="emit('refresh')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { UpdateTask } from '@/api/system-update'
import { getTaskStatusLabel, getTaskStatusType } from '../composables/sysUpdFmts'

interface TaskQuery {
  page: number
  page_size: number
}

/**
 * 系统更新任务表格组件
 */
const props = defineProps<{
  tasks: UpdateTask[]
  loading: boolean
  total: number
  query: TaskQuery
}>()

const emit = defineEmits<{
  rollback: [row: UpdateTask]
  cancel: [row: UpdateTask]
  refresh: []
}>()

void props
</script>

<style scoped>
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
