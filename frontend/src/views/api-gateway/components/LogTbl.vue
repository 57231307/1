<!--
  LogTbl.vue - API 调用日志列表
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-table v-loading="loading" :data="logs" stripe>
    <el-table-column prop="endpoint_path" label="接口路径" min-width="200" />
    <el-table-column prop="method" label="方法" width="80">
      <template #default="{ row }">
        <el-tag :type="METHOD_TYPE_MAP[row.method]" size="small">
          {{ row.method }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="status_code" label="状态码" width="80">
      <template #default="{ row }">
        <el-tag :type="row.status_code < 400 ? 'success' : 'danger'" size="small">
          {{ row.status_code }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="response_time" label="响应时间" width="100">
      <template #default="{ row }"> {{ row.response_time }}ms </template>
    </el-table-column>
    <el-table-column prop="ip_address" label="IP地址" width="140" />
    <el-table-column prop="user_name" label="用户" width="100" />
    <el-table-column prop="created_at" label="请求时间" width="160" />
    <el-table-column label="操作" width="100" fixed="right">
      <template #default="{ row }">
        <el-button type="primary" link size="small" @click="emit('view', row)">详情</el-button>
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
      @size-change="emit('page-change')"
      @current-change="emit('page-change')"
    />
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { ApiLog } from '@/api/api-gateway'
import { METHOD_TYPE_MAP } from '../composables/apiGwFmts'

interface LogQuery {
  page: number
  page_size: number
  keyword: string
  method: string
  status_code: string
}

const props = defineProps<{
  logs: ApiLog[]
  loading: boolean
  total: number
  query: LogQuery
}>()

const emit = defineEmits<{
  view: [row: ApiLog]
  'page-change': []
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
