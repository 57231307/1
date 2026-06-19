<!--
  EpTbl.vue - 接口管理列表
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-table v-loading="loading" :data="endpoints" stripe>
    <el-table-column prop="path" label="接口路径" min-width="200" />
    <el-table-column prop="method" label="方法" width="80">
      <template #default="{ row }">
        <el-tag :type="METHOD_TYPE_MAP[row.method]" size="small">
          {{ row.method }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="description" label="描述" min-width="150" />
    <el-table-column prop="module" label="模块" width="100" />
    <el-table-column prop="rate_limit" label="限流" width="80">
      <template #default="{ row }">
        {{ row.rate_limit ? `${row.rate_limit}/s` : '-' }}
      </template>
    </el-table-column>
    <el-table-column prop="timeout" label="超时(ms)" width="100" />
    <el-table-column prop="authentication" label="认证" width="80" align="center">
      <template #default="{ row }">
        <el-tag :type="row.authentication ? 'success' : 'info'" size="small">
          {{ row.authentication ? '是' : '否' }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="status" label="状态" width="80" align="center">
      <template #default="{ row }">
        <el-tag :type="EP_STATUS_TYPE_MAP[row.status]" size="small">
          {{ EP_STATUS_LABEL_MAP[row.status] }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column label="操作" width="150" fixed="right">
      <template #default="{ row }">
        <el-button type="primary" link size="small" @click="emit('edit', row)">编辑</el-button>
        <el-button type="danger" link size="small" @click="emit('delete', row)">删除</el-button>
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
import type { ApiEndpoint } from '@/api/api-gateway'
import { METHOD_TYPE_MAP, EP_STATUS_LABEL_MAP, EP_STATUS_TYPE_MAP } from '../composables/apiGwFmts'

interface EpQuery {
  page: number
  page_size: number
  keyword: string
  method: string
  status: string
}

const props = defineProps<{
  endpoints: ApiEndpoint[]
  loading: boolean
  total: number
  query: EpQuery
}>()

const emit = defineEmits<{
  edit: [row: ApiEndpoint]
  delete: [row: ApiEndpoint]
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
