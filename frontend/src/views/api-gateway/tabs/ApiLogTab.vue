<!--
  ApiLogTab.vue - API 网关调用日志 Tab
  来源：原 api-gateway/index.vue 中 logs tab
  拆分日期：2026-06-17 P1-3-Batch-5
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        v-model="localQuery.keyword"
        placeholder="搜索路径/客户端"
        style="width: 200px"
        clearable
        @clear="handleSearch"
        @keyup.enter="handleSearch"
      />
      <el-select v-model="localQuery.status" placeholder="状态码" clearable style="width: 120px">
        <el-option label="2xx 成功" value="2xx" />
        <el-option label="4xx 客户端" value="4xx" />
        <el-option label="5xx 服务端" value="5xx" />
      </el-select>
      <el-date-picker
        v-model="localQuery.date_range"
        type="daterange"
        range-separator="至"
        start-placeholder="开始日期"
        end-placeholder="结束日期"
        style="width: 260px"
      />
      <el-button type="primary" @click="handleSearch">
        <el-icon><Search /></el-icon>
        搜索
      </el-button>
    </div>

    <el-table v-loading="loading" :data="logs" stripe>
      <el-table-column prop="created_at" label="时间" width="160" />
      <el-table-column prop="path" label="接口路径" min-width="200" />
      <el-table-column prop="method" label="方法" width="80">
        <template #default="{ row }">
          <el-tag :type="methodTypeMap[row.method]" size="small">
            {{ row.method }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="status_code" label="状态码" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status_code)" size="small">
            {{ row.status_code }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="duration" label="耗时(ms)" width="100" align="right" />
      <el-table-column prop="client_ip" label="客户端 IP" width="140" />
      <el-table-column prop="api_key_name" label="密钥" width="150" />
      <el-table-column label="操作" width="100" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view-log', row)"
            >详情</el-button
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
        @current-change="(v: number) => emit('update:page', v)"
        @size-change="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { ApiLog } from '@/api/api-gateway'

export interface LogQuery {
  keyword: string
  status: string
  date_range: [Date, Date] | null
}

const props = defineProps<{
  logs: ApiLog[]
  loading: boolean
  total: number
  page: number
  pageSize: number
  // 批次 281：queryParams 类型放宽为 Record<string, unknown>，兼容 useTableApi 的 queryParams
  queryParams: Record<string, unknown>
  methodTypeMap: Record<string, string>
}>()

const emit = defineEmits<{
  fetch: []
  'update:page': [value: number]
  'update:page-size': [value: number]
  'view-log': [row: ApiLog]
  'update:queryParams': [value: LogQuery]
}>()

const localQuery = reactive<LogQuery>({
  keyword: '',
  status: '',
  date_range: null,
  ...(props.queryParams as Partial<LogQuery>),
})

watch(
  () => props.queryParams,
  newQuery => Object.assign(localQuery, newQuery),
  { deep: true }
)

// 批次 281：搜索时先同步筛选条件到父组件 queryParams，再触发 fetch 刷新
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

const getStatusType = (code: number) => {
  if (code >= 200 && code < 300) return 'success'
  if (code >= 400 && code < 500) return 'warning'
  if (code >= 500) return 'danger'
  return 'info'
}
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
