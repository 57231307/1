<!--
  SlowQueryView.vue - 慢查询审计查看页（P13 批 1 B-慢查询审计）
  任务编号: P13 批 1 G
  关联 spec: docs/superpowers/plans/2026-06-18-p13-batch1-comprehensive-plan.md §2.2
  功能：
  - 分页 + 多维筛选（时间范围 / 最小执行时间 / 关键词）
  - V2Table 表格展示
  - TOP 10 卡片展示（按最大平均执行时间排序）
  - 手动刷新按钮
-->
<template>
  <div class="slow-query-view">
    <!-- TOP 10 卡片 -->
    <el-card shadow="hover" class="top-card">
      <template #header>
        <div class="card-header">
          <span class="card-title">慢查询 TOP 10（{{ stats?.time_range || '近 7 天' }}）</span>
          <el-button type="primary" size="small" :loading="refreshing" @click="handleRefresh">
            <el-icon><Refresh /></el-icon>
            手动刷新
          </el-button>
        </div>
      </template>
      <el-row :gutter="12" v-loading="statsLoading">
        <el-col
          v-for="(item, idx) in stats?.top10 || []"
          :key="idx"
          :xs="24"
          :sm="12"
          :md="8"
          :lg="6"
          class="top-col"
        >
          <el-card class="top-item" shadow="hover">
            <div class="top-rank">#{{ idx + 1 }}</div>
            <div class="top-query" :title="item.query_text">
              {{ truncate(item.query_text, 60) }}
            </div>
            <div class="top-meta">
              <el-tag :type="getDurationTag(item.max_exec_time_ms)" size="small">
                {{ item.max_exec_time_ms.toFixed(1) }} ms
              </el-tag>
              <span class="top-calls">调用 {{ item.total_calls }} 次</span>
            </div>
          </el-card>
        </el-col>
        <el-col v-if="!statsLoading && (!stats || stats.top10.length === 0)">
          <el-empty description="暂无慢查询数据，请点击「手动刷新」触发采集" />
        </el-col>
      </el-row>
    </el-card>

    <!-- 筛选 + 表格 -->
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="filterForm" @submit.prevent="handleQuery">
        <el-form-item label="时间范围">
          <el-date-picker
            v-model="filterForm.dateRange"
            type="datetimerange"
            range-separator="至"
            start-placeholder="开始时间"
            end-placeholder="结束时间"
            value-format="YYYY-MM-DDTHH:mm:ss[Z]"
            style="width: 360px"
          />
        </el-form-item>
        <el-form-item label="最小执行时间">
          <el-input-number
            v-model="filterForm.min_duration"
            :min="0"
            :step="50"
            placeholder="毫秒"
            style="width: 140px"
          />
        </el-form-item>
        <el-form-item label="关键词">
          <el-input
            v-model="filterForm.keyword"
            placeholder="SQL 片段"
            clearable
            style="width: 240px"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <V2Table
        :columns="columns"
        :data="data"
        :loading="loading"
        :page="page"
        :page-size="pageSize"
        :total="total"
        :height="500"
        row-key="id"
        empty-text="暂无慢查询记录"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
/**
 * 慢查询审计查看页（P13 批 1 B-慢查询审计）
 * - 后端路由：/api/v1/erp/slow-queries（list / stats / refresh）
 */
import { ref, reactive, onMounted, h } from 'vue'
import { ElMessage, ElTag } from 'element-plus'
import { Search, Refresh } from '@element-plus/icons-vue'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import { useTableApi } from '@/composables/useTableApi'
import {
  getSlowQueryStats,
  refreshSlowQueries,
  type SlowQueryItem,
  type SlowQueryStatsResponse,
} from '@/api/slow-query'

// 慢查询统计响应
const stats = ref<SlowQueryStatsResponse | null>(null)
const statsLoading = ref(false)
const refreshing = ref(false)

// 筛选表单
const filterForm = reactive({
  dateRange: [] as string[],
  min_duration: undefined as number | undefined,
  keyword: '',
})

// 批次 267：接入 useTableApi，消除手写 page/pageSize/total/loading + loadData 重复
// API 返回 { items, total }，配置 listKey: 'items' 让 useTableApi 正确探测
const {
  data,
  loading,
  page,
  pageSize,
  total,
  refresh,
  setQueryParam,
} = useTableApi<SlowQueryItem>({
  url: '/slow-queries',
  listKey: 'items',
  onError: () => ElMessage.error('加载慢查询失败'),
})

// 表格列定义
const columns: ColumnDef<SlowQueryItem>[] = [
  { key: 'id', title: 'ID', width: 70 },
  { key: 'captured_at', title: '采集时间', width: 170, formatter: (row) => formatDateTime(row.captured_at) },
  {
    key: 'execution_time_ms',
    title: '平均耗时',
    width: 120,
    renderCell: (row) => {
      const ms = Number(row.execution_time_ms ?? 0)
      return h(
        ElTag,
        { type: getDurationTag(ms), size: 'small' },
        () => `${ms.toFixed(1)} ms`,
      )
    },
  },
  { key: 'calls', title: '调用次数', width: 100 },
  { key: 'rows_examined', title: '扫描行数', width: 110 },
  {
    key: 'query_text',
    title: 'SQL 文本',
    minWidth: 360,
    formatter: (row) => truncate(String(row.query_text ?? ''), 100),
  },
  { key: 'database_name', title: '数据库', width: 120 },
]

/**
 * 根据执行时间返回 el-tag 颜色（绿色 < 200ms / 黄色 < 500ms / 红色 >= 500ms）
 */
const getDurationTag = (ms: number): 'success' | 'warning' | 'danger' => {
  if (ms < 200) return 'success'
  if (ms < 500) return 'warning'
  return 'danger'
}

/**
 * 截断长字符串（前端展示用，避免长 SQL 撑爆布局）
 */
const truncate = (s: string, max: number): string => {
  if (!s) return ''
  return s.length > max ? s.slice(0, max) + '...' : s
}

/**
 * 批次 267：同步筛选条件到 useTableApi.queryParams
 * useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
 */
const syncQueryParams = () => {
  // 先清空旧筛选，再写入新值（避免上次筛选残留）
  setQueryParam('start_time', undefined)
  setQueryParam('end_time', undefined)
  setQueryParam('min_duration', undefined)
  setQueryParam('keyword', undefined)

  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    setQueryParam('start_time', filterForm.dateRange[0])
    setQueryParam('end_time', filterForm.dateRange[1])
  }
  if (filterForm.min_duration !== undefined && filterForm.min_duration !== null) {
    setQueryParam('min_duration', filterForm.min_duration)
  }
  if (filterForm.keyword.trim()) setQueryParam('keyword', filterForm.keyword.trim())
}

/**
 * 加载 TOP 10 统计（独立于 useTableApi，保留原 request 调用）
 */
const loadStats = async () => {
  statsLoading.value = true
  try {
    stats.value = await getSlowQueryStats()
  } catch (err) {
    // 统计接口失败不阻断列表展示
  } finally {
    statsLoading.value = false
  }
}

/**
 * 手动触发一次采集
 */
const handleRefresh = async () => {
  refreshing.value = true
  try {
    const res = await refreshSlowQueries()
    ElMessage.success(res.message)
    // 刷新完成后重新加载列表（useTableApi.refresh）+ 统计
    await Promise.all([refresh(), loadStats()])
  } catch (err) {
    ElMessage.error('手动刷新失败')
  } finally {
    refreshing.value = false
  }
}

/**
 * 查询按钮：同步筛选 + 重置到第一页 + 刷新
 */
const handleQuery = () => {
  syncQueryParams()
  page.value = 1
  refresh()
}

/**
 * 重置筛选条件
 */
const handleReset = () => {
  filterForm.dateRange = []
  filterForm.min_duration = undefined
  filterForm.keyword = ''
  syncQueryParams()
  page.value = 1
  refresh()
}

/**
 * 分页变化（useTableApi 自动 watch 重载，此处仅更新 page 值）
 */
const handlePageChange = (p: number) => {
  page.value = p
}

/**
 * 每页大小变化（useTableApi 自动 watch 重载，切回第一页）
 */
const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

/**
 * 格式化日期时间
 */
const formatDateTime = (v: string | null | undefined): string => {
  if (!v) return '-'
  const d = new Date(v)
  if (isNaN(d.getTime())) return v
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

// 批次 267：useTableApi 构造时自动初始加载列表，onMounted 仅加载统计
onMounted(() => {
  loadStats()
})
</script>

<style scoped>
.slow-query-view {
  padding: 16px;
}
.top-card {
  margin-bottom: 16px;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.card-title {
  font-weight: 600;
  font-size: 14px;
}
.top-col {
  margin-bottom: 12px;
}
.top-item {
  position: relative;
}
.top-rank {
  position: absolute;
  top: 8px;
  right: 12px;
  font-size: 18px;
  font-weight: 700;
  color: #f56c6c;
}
.top-query {
  font-family: monospace;
  font-size: 12px;
  line-height: 1.4;
  color: #303133;
  margin: 6px 0 8px;
  word-break: break-all;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  height: 50px;
}
.top-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
}
.top-calls {
  color: #909399;
}
.filter-card {
  margin-bottom: 16px;
}
.table-card {
  margin-bottom: 16px;
}
</style>
