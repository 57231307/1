<!--
  DiTaskTbl.vue - 数据导入任务列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-select
        :model-value="query.status"
        placeholder="状态"
        clearable
        style="width: 120px"
        @update:model-value="(v: string) => (query.status = v)"
      >
        <el-option label="待处理" value="pending" />
        <el-option label="处理中" value="processing" />
        <el-option label="已完成" value="completed" />
        <el-option label="失败" value="failed" />
      </el-select>
      <el-button type="primary" @click="emit('search')">
        <el-icon><Search /></el-icon>
        搜索
      </el-button>
    </div>

    <el-table v-loading="loading" :data="data" stripe>
      <el-table-column prop="task_code" label="任务编号" width="140" />
      <el-table-column prop="template_name" label="导入模板" width="150" />
      <el-table-column prop="file_name" label="文件名" min-width="180" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="TASK_STATUS_TYPE_MAP[row.status]" size="small">
            {{ TASK_STATUS_MAP[row.status] }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="progress" label="进度" width="120">
        <template #default="{ row }">
          <el-progress
            :percentage="row.progress"
            :status="row.status === 'failed' ? 'exception' : undefined"
          />
        </template>
      </el-table-column>
      <el-table-column prop="total_rows" label="总行数" width="80" />
      <el-table-column prop="success_rows" label="成功" width="80" />
      <el-table-column prop="failed_rows" label="失败" width="80" />
      <el-table-column prop="created_by_name" label="创建人" width="100" />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'failed'"
            type="primary"
            link
            size="small"
            @click="emit('retry', row)"
            >重试</el-button
          >
          <el-button
            v-if="row.status === 'pending' || row.status === 'processing'"
            type="danger"
            link
            size="small"
            @click="emit('cancel', row)"
            >取消</el-button
          >
          <el-button
            v-if="row.failed_rows > 0"
            type="warning"
            link
            size="small"
            @click="emit('download-log', row)"
            >错误日志</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="query.page"
        :page-size="query.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="onPageChange"
        @update:page-size="onSizeChange"
        @size-change="emit('search')"
        @current-change="emit('search')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Search } from '@element-plus/icons-vue'
import type { ImportTask } from '@/api/data-import'
import { TASK_STATUS_MAP, TASK_STATUS_TYPE_MAP } from '../composables/diFmts'

// 查询参数类型
interface TaskQuery {
  page: number
  page_size: number
  status: string
}

/**
 * 任务列表组件（含过滤栏）
 */
defineProps<{
  // 任务数据
  data: ImportTask[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 查询参数
  query: TaskQuery
}>()

const emit = defineEmits<{
  search: []
  retry: [row: ImportTask]
  cancel: [row: ImportTask]
  'download-log': [row: ImportTask]
  // 分页
  'update:page': [v: number]
  'update:size': [v: number]
}>()

/** 页码变更 */
const onPageChange = (page: number) => {
  emit('update:page', page)
}

/** 每页大小变更 */
const onSizeChange = (size: number) => {
  emit('update:size', size)
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
