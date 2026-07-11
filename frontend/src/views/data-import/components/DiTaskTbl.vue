<!--
  DiTaskTbl.vue - 数据导入任务列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  批次 289：改造为 localQuery + handleSearch 模式，接入 useTableApi queryParams
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-select
        v-model="localQuery.status"
        placeholder="状态"
        clearable
        style="width: 120px"
      >
        <el-option label="待处理" value="pending" />
        <el-option label="处理中" value="processing" />
        <el-option label="已完成" value="completed" />
        <el-option label="失败" value="failed" />
      </el-select>
      <el-button type="primary" @click="handleSearch">
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
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { ImportTask } from '@/api/data-import'
import { TASK_STATUS_MAP, TASK_STATUS_TYPE_MAP } from '../composables/diFmts'

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
