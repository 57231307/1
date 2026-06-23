<!--
  DiTaskTbl.vue - 数据导入任务列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 reactive 镜像 + watch 同步 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-select
        :model-value="localParams.status"
        placeholder="状态"
        clearable
        style="width: 120px"
        @update:model-value="(v: string) => { localParams.status = v; syncToParent() }"
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
        :current-page="localParams.page"
        :page-size="localParams.page_size"
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
import { reactive, watch } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { ImportTask } from '@/api/data-import'
import { TASK_STATUS_MAP, TASK_STATUS_TYPE_MAP } from '../composables/diFmts'
import type { TaskQuery } from '../composables/useDi'

// 查询参数默认值（父组件未传入时使用）
const DEFAULT_PARAMS: TaskQuery = {
  page: 1,
  page_size: 20,
  status: '',
}

/**
 * 任务列表组件（含过滤栏）
 */
const props = defineProps<{
  // 任务数据
  data: ImportTask[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 查询参数（由父组件管理，子组件通过 emit('update:params') 整体回写）
  params?: TaskQuery
}>()

const emit = defineEmits<{
  search: []
  retry: [row: ImportTask]
  cancel: [row: ImportTask]
  'download-log': [row: ImportTask]
  // 整体回写查询参数（父组件监听后 Object.assign 到自己的 params）
  'update:params': [params: TaskQuery]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localParams = reactive<TaskQuery>({
  ...(props.params ?? DEFAULT_PARAMS),
})

// 父组件传参变化时同步到本地（如父组件重置分页/过滤条件）
watch(
  () => props.params,
  newParams => {
    if (newParams) Object.assign(localParams, newParams)
  },
  { deep: true },
)

/** 同步本地到父组件（深拷贝避免外部引用被意外修改） */
const syncToParent = () => {
  emit('update:params', { ...localParams })
}

/** 页码变更 */
const onPageChange = (page: number) => {
  localParams.page = page
  syncToParent()
}

/** 每页大小变更 */
const onSizeChange = (size: number) => {
  localParams.page_size = size
  syncToParent()
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
