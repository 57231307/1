<!--
  ApiLogTab.vue - API 网关调用日志 Tab
  来源：原 api-gateway/index.vue 中 logs tab
  拆分日期：2026-06-17 P1-3-Batch-5
  迁移说明：el-table + el-pagination 替换为 V2Table（基于 el-table-v2 虚拟滚动），
            保留筛选区域与全部列/自定义渲染，分页改由 V2Table 内置。
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

    <V2Table
      :columns="columns"
      :data="logs"
      :loading="loading"
      :page="page"
      :page-size="pageSize"
      :page-sizes="[10, 20, 50, 100]"
      :total="total"
      :height="600"
      @page-change="(v: number) => emit('update:page', v)"
      @size-change="(v: number) => emit('update:page-size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { computed, h, reactive, watch } from 'vue'
import { ElTag, ElButton } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import type { ApiLog } from '@/api/api-gateway'

export interface LogQuery {
  keyword: string
  status: string
  date_range: [Date, Date] | null
}

// ElTag 类型联合（与 element-plus TagProps type 对齐，避免将 string 直接传入 type）
type TagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'

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

/**
 * 列定义（computed：方法列的 el-tag type 依赖 props.methodTypeMap）
 * - 方法列：el-tag，type 来自 props.methodTypeMap
 * - 状态码列：el-tag，type 来自 getStatusType
 * - 操作列：详情按钮（fixed right）
 */
const columns = computed<ColumnDef<ApiLog>[]>(() => [
  { key: 'created_at', title: '时间', width: 160 },
  { key: 'path', title: '接口路径', minWidth: 200 },
  {
    key: 'method',
    title: '方法',
    width: 80,
    renderCell: row =>
      h(
        ElTag,
        { type: (props.methodTypeMap[row.method] ?? 'info') as TagType, size: 'small' },
        { default: () => row.method }
      ),
  },
  {
    key: 'status_code',
    title: '状态码',
    width: 100,
    align: 'center',
    renderCell: row =>
      h(
        ElTag,
        { type: getStatusType(row.status_code), size: 'small' },
        { default: () => row.status_code }
      ),
  },
  { key: 'duration', title: '耗时(ms)', width: 100, align: 'right' },
  { key: 'client_ip', title: '客户端 IP', width: 140 },
  { key: 'api_key_name', title: '密钥', width: 150 },
  {
    key: '__actions__',
    title: '操作',
    width: 100,
    fixed: 'right',
    renderCell: row =>
      h(
        ElButton,
        { type: 'primary', link: true, size: 'small', onClick: () => emit('view-log', row) },
        { default: () => '详情' }
      ),
  },
])
</script>

<style scoped>
.filter-container {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}
</style>
