<script setup lang="ts">
import { ref } from 'vue'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElInput,
  ElSelect,
  ElDatePicker,
  ElMessage,
  ElRow,
  ElCol,
  ElCard,
  ElTabs,
  ElTabPane,
  ElStatistic,
  ElDescriptions,
} from 'element-plus'
import { PieChart, Clock, AlarmClock } from '@element-plus/icons-vue'
import { getDashboardStats, type AuditStats, type AuditLog } from '@/api/omniAudit'
import type { ApiResponse } from '@/types/api'
import { useTableApi } from '@/composables/useTableApi'

const activeTab = ref('dashboard')
const stats = ref<AuditStats | null>(null)
// stats 独立加载状态（与 logs 的 useTableApi.loading 分离，避免双 tab 切换时互相干扰）
const statsLoading = ref(false)

const searchForm = ref({
  user_id: '',
  event_type: '',
  resource: '',
  action: '',
  status: '',
  start_time: '',
  end_time: '',
})

// 批次 273：接入 useTableApi，消除手写 logs/total/loading/pagination/loadLogs 重复
// 修复 0-based 分页 bug：原 page-1 传 0 被后端 clamp(1,1000) 修正为 1，page=2 时传 1 offset=0，分页错乱
// useTableApi 使用 1-based 分页，与后端 page.unwrap_or(1).clamp(1,1000) + page.saturating_sub(1)*page_size 一致
const {
  data: logs,
  loading,
  page,
  pageSize,
  total,
  refresh: loadLogs,
  setQueryParam,
} = useTableApi<AuditLog>({
  url: '/finance/audit/search',
  listKey: 'items',
  onError: () => ElMessage.error('加载日志失败'),
})

const viewDialogVisible = ref(false)
const viewData = ref<AuditLog | null>(null)

const statusOptions = [
  { label: '全部', value: '' },
  { label: '成功', value: 'SUCCESS' },
  { label: '失败', value: 'FAILED' },
]

const getStatusLabel = (value: string) => {
  return statusOptions.find(s => s.value === value)?.label || value
}

const getStatusClass = (value: string) => {
  return value === 'SUCCESS' ? 'status-success' : 'status-failed'
}

const loadStats = async () => {
  statsLoading.value = true
  try {
    // v11 批次 146 P1-3 修复：拦截器已返回 ApiResponse 完整对象，
    // res.data 即业务数据（AuditStats），无需 res.data!.data 双层访问
    const res = await getDashboardStats()
    stats.value = (res as ApiResponse<AuditStats> | undefined)?.data ?? null
  } catch (error) {
    ElMessage.error('加载统计数据失败')
  } finally {
    statsLoading.value = false
  }
}

// 批次 273：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadLogs
const syncQueryParams = () => {
  setQueryParam('user_id', searchForm.value.user_id ? Number(searchForm.value.user_id) : undefined)
  setQueryParam('event_type', searchForm.value.event_type || undefined)
  setQueryParam('resource', searchForm.value.resource || undefined)
  setQueryParam('action', searchForm.value.action || undefined)
  setQueryParam('status', searchForm.value.status || undefined)
  setQueryParam('start_time', searchForm.value.start_time || undefined)
  setQueryParam('end_time', searchForm.value.end_time || undefined)
}

const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  loadLogs()
}

const handleReset = () => {
  searchForm.value = {
    user_id: '',
    event_type: '',
    resource: '',
    action: '',
    status: '',
    start_time: '',
    end_time: '',
  }
  syncQueryParams()
  page.value = 1
  loadLogs()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handlePageSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

const openViewDialog = (row: AuditLog) => {
  viewData.value = row
  viewDialogVisible.value = true
}

loadStats()
// 批次 273：useTableApi 构造时自动初始加载 logs，无需 setup 顶层调用 loadLogs
</script>

<template>
  <div class="app-container">
    <ElTabs v-model="activeTab" @tab-change="activeTab === 'dashboard' ? loadStats() : loadLogs()">
      <ElTabPane label="审计大屏" name="dashboard">
        <div class="stats-grid">
          <ElCard class="stat-card">
            <div class="stat-icon total">
              <PieChart />
            </div>
            <ElStatistic title="总事件数" :value="stats?.total_events || 0" />
          </ElCard>
          <ElCard class="stat-card">
            <div class="stat-icon today">
              <Clock />
            </div>
            <ElStatistic title="今日事件" :value="stats?.today_events || 0" />
          </ElCard>
          <ElCard class="stat-card">
            <div class="stat-icon error">
              <AlarmClock />
            </div>
            <ElStatistic title="错误数" :value="stats?.error_count || 0" />
          </ElCard>
          <ElCard class="stat-card">
            <div class="stat-icon avg">
              <Clock />
            </div>
            <ElStatistic title="平均耗时(ms)" :value="stats?.avg_duration_ms || 0" />
          </ElCard>
        </div>

        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElCard title="热门资源" class="chart-card">
              <ElTable :data="stats?.top_resources || []" border style="width: 100%" aria-label="热门资源列表">
                <ElTableColumn prop="name" label="资源名称" />
                <ElTableColumn prop="count" label="访问次数" align="right" />
              </ElTable>
            </ElCard>
          </ElCol>
          <ElCol :span="12">
            <ElCard title="活跃用户" class="chart-card">
              <ElTable :data="stats?.top_users || []" border style="width: 100%" aria-label="活跃用户列表">
                <ElTableColumn prop="name" label="用户名称" />
                <ElTableColumn prop="count" label="操作次数" align="right" />
              </ElTable>
            </ElCard>
          </ElCol>
        </ElRow>
      </ElTabPane>

      <ElTabPane label="审计日志" name="logs">
        <div class="filter-container">
          <ElRow :gutter="20">
            <ElCol :span="6">
              <ElInput
                v-model="searchForm.user_id"
                placeholder="用户ID"
                class="filter-item"
                @keyup.enter="handleSearch"
              />
            </ElCol>
            <ElCol :span="6">
              <ElInput
                v-model="searchForm.resource"
                placeholder="资源"
                class="filter-item"
                @keyup.enter="handleSearch"
              />
            </ElCol>
            <ElCol :span="6">
              <ElInput
                v-model="searchForm.action"
                placeholder="操作"
                class="filter-item"
                @keyup.enter="handleSearch"
              />
            </ElCol>
            <ElCol :span="6">
              <ElSelect v-model="searchForm.status" placeholder="状态" class="filter-item">
                <ElOption
                  v-for="s in statusOptions"
                  :key="s.value"
                  :label="s.label"
                  :value="s.value"
                />
              </ElSelect>
            </ElCol>
          </ElRow>
          <ElRow :gutter="20" style="margin-top: 10px">
            <ElCol :span="10">
              <ElDatePicker
                v-model="searchForm.start_time"
                type="datetime"
                placeholder="开始时间"
                class="filter-item"
              />
            </ElCol>
            <ElCol :span="10">
              <ElDatePicker
                v-model="searchForm.end_time"
                type="datetime"
                placeholder="结束时间"
                class="filter-item"
              />
            </ElCol>
            <ElCol :span="4">
              <div class="filter-actions">
                <ElButton type="primary" @click="handleSearch">查询</ElButton>
                <ElButton @click="handleReset">重置</ElButton>
              </div>
            </ElCol>
          </ElRow>
        </div>

        <ElTable
          :data="logs"
          :loading="loading"
          border
          fit
          highlight-current-row
          style="width: 100%"
          aria-label="审计日志列表"
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="user_name" label="用户" width="100" />
          <ElTableColumn prop="event_type" label="事件类型" width="120" />
          <ElTableColumn prop="event_name" label="事件名称" width="150" />
          <ElTableColumn prop="resource" label="资源" width="120" />
          <ElTableColumn prop="action" label="操作" width="100" />
          <ElTableColumn prop="status" label="状态" width="100">
            <template #default="scope">
              <span :class="['status-tag', getStatusClass(scope.row.status)]">
                {{ getStatusLabel(scope.row.status) }}
              </span>
            </template>
          </ElTableColumn>
          <ElTableColumn prop="duration_ms" label="耗时(ms)" width="100" align="right" />
          <ElTableColumn prop="created_at" label="时间" width="180" />
          <ElTableColumn label="操作" width="100" align="center">
            <template #default="scope">
              <ElButton size="small" @click="openViewDialog(scope.row as AuditLog)">详情</ElButton>
            </template>
          </ElTableColumn>
        </ElTable>

        <div class="pagination-wrapper" style="margin-top: 16px; text-align: right">
          <ElPagination
            v-model:current-page="page"
            v-model:page-size="pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="total"
            layout="total, sizes, prev, pager, next, jumper"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
            aria-label="审计日志列表分页"
          />
        </div>
      </ElTabPane>
    </ElTabs>

    <ElDialog
      title="审计日志详情"
      :visible="viewDialogVisible"
      width="800px"
      aria-label="审计日志详情"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <ElDescriptions :column="2" border>
          <ElDescriptionsItem label="ID">{{ viewData.id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="追踪ID">{{ viewData.trace_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="用户ID">{{ viewData.user_id }}</ElDescriptionsItem>
          <ElDescriptionsItem label="用户名称">{{ viewData.user_name || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="事件类型">{{ viewData.event_type }}</ElDescriptionsItem>
          <ElDescriptionsItem label="事件名称">{{ viewData.event_name }}</ElDescriptionsItem>
          <ElDescriptionsItem label="资源">{{ viewData.resource }}</ElDescriptionsItem>
          <ElDescriptionsItem label="操作">{{ viewData.action }}</ElDescriptionsItem>
          <ElDescriptionsItem label="状态">{{
            getStatusLabel(viewData.status)
          }}</ElDescriptionsItem>
          <ElDescriptionsItem label="耗时(ms)">{{ viewData.duration_ms }}</ElDescriptionsItem>
          <ElDescriptionsItem label="IP地址">{{ viewData.ip_address || '-' }}</ElDescriptionsItem>
          <ElDescriptionsItem label="创建时间">{{ viewData.created_at }}</ElDescriptionsItem>
        </ElDescriptions>
        <div v-if="viewData.payload" style="margin-top: 20px">
          <h4>请求参数</h4>
          <pre class="payload-pre">{{ JSON.stringify(viewData.payload, null, 2) }}</pre>
        </div>
        <div v-if="viewData.error_msg" style="margin-top: 20px">
          <h4>错误信息</h4>
          <div class="error-box">{{ viewData.error_msg }}</div>
        </div>
      </div>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 20px;
  margin-bottom: 20px;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 20px;
}

.stat-icon {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
}

.stat-icon.total {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.today {
  background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
  color: white;
}

.stat-icon.error {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.avg {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: white;
}

.chart-card {
  margin-bottom: 20px;
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  display: flex;
  gap: 10px;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.status-success {
  background: #f0f9eb;
  color: #67c23a;
}

.status-failed {
  background: #fef0f0;
  color: #f56c6c;
}

.payload-pre {
  background: #f5f7fa;
  padding: 15px;
  border-radius: 4px;
  font-size: 12px;
  max-height: 300px;
  overflow-y: auto;
}

.error-box {
  background: #fef0f0;
  padding: 15px;
  border-radius: 4px;
  color: #f56c6c;
  font-size: 14px;
}
</style>
