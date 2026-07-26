<!--
  DataImportTaskTable.vue - 数据导入任务列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  批次 289：改造为 localQuery + handleSearch 模式，接入 useTableApi queryParams
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-select
        v-model="localQuery.status"
        :placeholder="t('dataImport.taskTable.statusPlaceholder')"
        clearable
        style="width: 120px"
      >
        <el-option :label="t('dataImport.taskTable.statusPending')" value="pending" />
        <el-option :label="t('dataImport.taskTable.statusProcessing')" value="processing" />
        <el-option :label="t('dataImport.taskTable.statusCompleted')" value="completed" />
        <el-option :label="t('dataImport.taskTable.statusFailed')" value="failed" />
      </el-select>
      <el-button type="primary" @click="handleSearch">
        <el-icon><Search /></el-icon>
        {{ t('dataImport.taskTable.search') }}
      </el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="data"
      stripe
      :aria-label="t('dataImport.taskTable.ariaLabel')"
    >
      <el-table-column prop="task_code" :label="t('dataImport.taskTable.taskCode')" width="140" />
      <el-table-column
        prop="template_name"
        :label="t('dataImport.taskTable.templateName')"
        width="150"
      />
      <el-table-column
        prop="file_name"
        :label="t('dataImport.taskTable.fileName')"
        min-width="180"
      />
      <el-table-column
        prop="status"
        :label="t('dataImport.taskTable.status')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="TASK_STATUS_TYPE_MAP[row.status]" size="small">
            {{ getTaskStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="progress" :label="t('dataImport.taskTable.progress')" width="120">
        <template #default="{ row }">
          <el-progress
            :percentage="row.progress"
            :status="row.status === 'failed' ? 'exception' : undefined"
          />
        </template>
      </el-table-column>
      <el-table-column prop="total_rows" :label="t('dataImport.taskTable.totalRows')" width="80" />
      <el-table-column
        prop="success_rows"
        :label="t('dataImport.taskTable.successRows')"
        width="80"
      />
      <el-table-column
        prop="failed_rows"
        :label="t('dataImport.taskTable.failedRows')"
        width="80"
      />
      <el-table-column
        prop="created_by_name"
        :label="t('dataImport.taskTable.createdBy')"
        width="100"
      />
      <el-table-column prop="created_at" :label="t('dataImport.taskTable.createdAt')" width="160" />
      <el-table-column :label="t('dataImport.taskTable.operation')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'failed'"
            type="primary"
            link
            size="small"
            @click="emit('retry', row)"
            >{{ t('dataImport.taskTable.retry') }}</el-button
          >
          <el-button
            v-if="row.status === 'pending' || row.status === 'processing'"
            type="danger"
            link
            size="small"
            @click="emit('cancel', row)"
            >{{ t('dataImport.taskTable.cancel') }}</el-button
          >
          <el-button
            v-if="row.failed_rows > 0"
            type="warning"
            link
            size="small"
            @click="emit('download-log', row)"
            >{{ t('dataImport.taskTable.errorLog') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('dataImport.taskTable.paginationAriaLabel')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search } from '@element-plus/icons-vue'
import type { ImportTask } from '@/api/data-import'
import { TASK_STATUS_TYPE_MAP } from '../composables/diFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 任务列表组件（含过滤栏）
 * 接收父组件传入的 queryParams + page/pageSize，通过 emit 同步变更
 * 查询时先同步 queryParams 再触发 fetch
 */
const props = defineProps<{
  // 任务数据
  data: ImportTask[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 查询条件（由父组件 useTableApi 管理，类型放宽为 Record 兼容 useTableApi）
  queryParams: Record<string, unknown>
  // 当前页码
  page: number
  // 每页大小
  pageSize: number
}>()

const emit = defineEmits<{
  retry: [row: ImportTask]
  cancel: [row: ImportTask]
  'download-log': [row: ImportTask]
  // 触发查询（父组件监听后调用 handleTaskSearch 重置页码并加载）
  fetch: []
  // 同步查询条件到父组件
  'update:queryParams': [params: Record<string, unknown>]
  // 分页变化（由 useTableApi watch 自动加载）
  'update:page': [page: number]
  'update:page-size': [pageSize: number]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQuery = reactive({
  status: (props.queryParams.status as string) ?? '',
})

/** 查询：先同步筛选条件到父组件，再触发 fetch */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/**
 * 任务状态标签映射（基于 i18n）
 */
const getTaskStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('dataImport.taskTable.statusPending'),
    processing: t('dataImport.taskTable.statusProcessing'),
    completed: t('dataImport.taskTable.statusCompleted'),
    failed: t('dataImport.taskTable.statusFailed'),
  }
  return map[status] || status
}
</script>

<style scoped>
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
