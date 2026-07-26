<!--
  SystemUpdateTaskTab.vue - 系统更新任务 Tab
  来源：原 system-update/index.vue 中 tasks tab
  拆分日期：2026-06-17 P1-3-Batch-5
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="tasks" stripe :aria-label="t('systemUpdate.taskTab.tableAriaLabel')">
      <el-table-column prop="task_code" :label="t('systemUpdate.taskTab.columnTaskCode')" width="140" />
      <el-table-column prop="from_version" :label="t('systemUpdate.taskTab.columnFromVersion')" width="100" />
      <el-table-column prop="to_version" :label="t('systemUpdate.taskTab.columnToVersion')" width="100" />
      <el-table-column prop="status" :label="t('systemUpdate.taskTab.columnStatus')" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="taskStatusTypeMap[row.status]" size="small">
            {{ getTaskStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="progress" :label="t('systemUpdate.taskTab.columnProgress')" width="150">
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
        :label="t('systemUpdate.taskTab.columnErrorMessage')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column prop="started_at" :label="t('systemUpdate.taskTab.columnStartedAt')" width="160" />
      <el-table-column prop="completed_at" :label="t('systemUpdate.taskTab.columnCompletedAt')" width="160" />
      <el-table-column :label="t('systemUpdate.taskTab.columnActions')" width="150" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'completed'"
            type="warning"
            link
            size="small"
            @click="emit('rollback', row)"
            >{{ t('systemUpdate.taskTab.buttonRollback') }}</el-button
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
            >{{ t('systemUpdate.taskTab.buttonCancel') }}</el-button
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
        :aria-label="t('systemUpdate.taskTab.paginationAriaLabel')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { type UpdateTask } from '@/api/system-update'

const { t } = useI18n({ useScope: 'global' })

defineProps<{
  tasks: UpdateTask[]
  loading: boolean
  total: number
  page: number
  pageSize: number
  taskStatusTypeMap: Record<string, string>
}>()

const emit = defineEmits<{
  rollback: [row: UpdateTask]
  cancel: [row: UpdateTask]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()

/** 更新任务状态 → i18n 标签（语言切换响应） */
const getTaskStatusLabel = (status: string): string => {
  switch (status) {
    case 'pending':
      return t('systemUpdate.common.taskStatusPending')
    case 'downloading':
      return t('systemUpdate.common.taskStatusDownloading')
    case 'downloaded':
      return t('systemUpdate.common.taskStatusDownloaded')
    case 'installing':
      return t('systemUpdate.common.taskStatusInstalling')
    case 'completed':
      return t('systemUpdate.common.taskStatusCompleted')
    case 'failed':
      return t('systemUpdate.common.taskStatusFailed')
    case 'rolled_back':
      return t('systemUpdate.common.taskStatusRolledBack')
    default:
      return status
  }
}
</script>

<style scoped>
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
