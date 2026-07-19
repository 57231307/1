<!--
  SystemUpdateTaskTab.vue - 系统更新任务 Tab
  来源：原 system-update/index.vue 中 tasks tab
  拆分日期：2026-06-17 P1-3-Batch-5
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="tasks" stripe aria-label="系统更新任务列表">
      <el-table-column prop="task_code" label="任务编号" width="140" />
      <el-table-column prop="from_version" label="原版本" width="100" />
      <el-table-column prop="to_version" label="目标版本" width="100" />
      <el-table-column prop="status" label="状态" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="taskStatusTypeMap[row.status]" size="small">
            {{ taskStatusMap[row.status] }}
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
            @click="emit('rollback', row)"
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
            @click="emit('cancel', row)"
            >取消</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
        aria-label="系统更新任务列表分页"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { type UpdateTask } from '@/api/system-update'

defineProps<{
  tasks: UpdateTask[]
  loading: boolean
  total: number
  page: number
  pageSize: number
  taskStatusTypeMap: Record<string, string>
  taskStatusMap: Record<string, string>
}>()

const emit = defineEmits<{
  rollback: [row: UpdateTask]
  cancel: [row: UpdateTask]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()
</script>

<style scoped>
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
