<!--
  ApiEndpointTab.vue - API 网关接口管理 Tab
  来源：原 api-gateway/index.vue 中 endpoints tab
  拆分日期：2026-06-17 P1-3-Batch-5
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        v-model="localQuery.keyword"
        placeholder="搜索接口路径/描述"
        style="width: 200px"
        clearable
        @clear="emit('fetch')"
        @keyup.enter="emit('fetch')"
      />
      <el-select
        v-model="localQuery.method"
        placeholder="请求方法"
        clearable
        style="width: 120px"
      >
        <el-option label="GET" value="GET" />
        <el-option label="POST" value="POST" />
        <el-option label="PUT" value="PUT" />
        <el-option label="DELETE" value="DELETE" />
        <el-option label="PATCH" value="PATCH" />
      </el-select>
      <el-select v-model="localQuery.status" placeholder="状态" clearable style="width: 120px">
        <el-option label="启用" value="active" />
        <el-option label="停用" value="inactive" />
        <el-option label="废弃" value="deprecated" />
      </el-select>
      <el-button type="primary" @click="emit('fetch')">
        <el-icon><Search /></el-icon>
        搜索
      </el-button>
      <el-button type="primary" @click="emit('new-endpoint')">
        <el-icon><Plus /></el-icon>
        新建接口
      </el-button>
    </div>

    <el-table v-loading="loading" :data="endpoints" stripe>
      <el-table-column prop="path" label="接口路径" min-width="200" />
      <el-table-column prop="method" label="方法" width="80">
        <template #default="{ row }">
          <el-tag :type="methodTypeMap[row.method]" size="small">
            {{ row.method }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="150" />
      <el-table-column prop="version" label="版本" width="80" />
      <el-table-column prop="rate_limit" label="限流/分钟" width="100" align="right" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="statusTypeMap[row.status]" size="small">
            {{ statusMap[row.status] }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="180" fixed="right">
        <template #default="{ row }">
          <el-button v-permission="'api_endpoint:update'" type="primary" link size="small" @click="emit('edit-endpoint', row)"
            >编辑</el-button
          >
          <el-button v-permission="'api_endpoint:delete'" type="danger" link size="small" @click="emit('delete-endpoint', row)"
            >删除</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        v-model:current-page="localQuery.page"
        v-model:page-size="localQuery.page_size"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('fetch')"
        @current-change="emit('fetch')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { Search, Plus } from '@element-plus/icons-vue'
import type { ApiEndpoint } from '@/api/api-gateway'

export interface EndpointQuery {
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
  queryParams: EndpointQuery
  methodTypeMap: Record<string, string>
  statusTypeMap: Record<string, string>
  statusMap: Record<string, string>
}>()

const emit = defineEmits<{
  fetch: []
  'new-endpoint': []
  'edit-endpoint': [row: ApiEndpoint]
  'delete-endpoint': [row: ApiEndpoint]
  'update:queryParams': [value: EndpointQuery]
}>()

const localQuery = reactive<EndpointQuery>({ ...props.queryParams })

watch(
  () => props.queryParams,
  newQuery => Object.assign(localQuery, newQuery),
  { deep: true }
)
</script>

<style scoped>
.filter-container {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}

.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
