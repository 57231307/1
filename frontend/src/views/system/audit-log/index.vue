<!--
  AuditLogView.vue - 审计日志查看页（P13 批 1 P3-2）
  任务编号: P13 批 1 H
  关联 spec: docs/superpowers/plans/2026-06-18-p13-batch1-comprehensive-plan.md §2.1
  功能：
  - 分页 + 多维筛选（时间范围 / 用户 / 操作类型 / 严重级别 / 资源类型 / 关键字）
  - V2Table 列表展示
  - 详情抽屉（el-drawer）展示 before/after 差异 JSON
  - CSV 导出按钮
-->
<template>
  <div class="audit-log-view">
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
        <el-form-item label="操作类型">
          <el-select
            v-model="filterForm.operation_type"
            placeholder="全部"
            clearable
            style="width: 160px"
          >
            <el-option
              v-for="opt in OP_TYPE_OPTIONS"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="严重级别">
          <el-select
            v-model="filterForm.severity"
            placeholder="全部"
            clearable
            style="width: 140px"
          >
            <el-option
              v-for="opt in SEVERITY_OPTIONS"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="资源类型">
          <el-input
            v-model="filterForm.resource_type"
            placeholder="如 user / order"
            clearable
            style="width: 160px"
          />
        </el-form-item>
        <el-form-item label="请求 ID">
          <el-input
            v-model="filterForm.request_id"
            placeholder="trace_id"
            clearable
            style="width: 200px"
          />
        </el-form-item>
        <el-form-item label="关键字">
          <el-input
            v-model="filterForm.keyword"
            placeholder="资源 ID / 名称 / 描述"
            clearable
            style="width: 180px"
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
          <el-button type="success" @click="handleExport">
            <el-icon><Download /></el-icon>
            导出 CSV
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
        :height="600"
        row-key="id"
        empty-text="暂无审计日志"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
        @row-click="handleRowClick"
      />
    </el-card>

    <!-- 详情抽屉：展示 before/after 差异快照 -->
    <el-drawer
      v-model="detailVisible"
      title="审计日志详情"
      size="60%"
      direction="rtl"
      :destroy-on-close="true"
    >
      <div v-if="currentDetail" class="detail-content">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="日志 ID">{{ currentDetail.id }}</el-descriptions-item>
          <el-descriptions-item label="租户">{{ currentDetail.tenant_id ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="操作人">
            {{ currentDetail.username ?? '-' }} (#{{ currentDetail.user_id ?? '-' }})
          </el-descriptions-item>
          <el-descriptions-item label="操作时间">
            {{ formatDateTime(currentDetail.created_at) }}
          </el-descriptions-item>
          <el-descriptions-item label="操作类型">
            <el-tag :type="OP_TYPE_TAG[currentDetail.operation_type ?? ''] ?? 'info'" size="small">
              {{ OP_TYPE_LABELS[currentDetail.operation_type ?? ''] ?? currentDetail.operation_type ?? '-' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="严重级别">
            <el-tag :type="SEVERITY_TAG[currentDetail.severity ?? ''] ?? 'info'" size="small">
              {{ currentDetail.severity ?? '-' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="资源类型">{{ currentDetail.resource_type ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="资源 ID">{{ currentDetail.resource_id ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="资源名称">{{ currentDetail.resource_name ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="请求方法">{{ currentDetail.request_method ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="请求路径" :span="2">
            {{ currentDetail.request_path ?? '-' }}
          </el-descriptions-item>
          <el-descriptions-item label="请求追踪">{{ currentDetail.request_id ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="客户端 IP">{{ currentDetail.ip_address ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="User-Agent" :span="2">
            <el-text line-clamp="2">{{ currentDetail.user_agent ?? '-' }}</el-text>
          </el-descriptions-item>
          <el-descriptions-item label="操作描述" :span="2">
            {{ currentDetail.description ?? '-' }}
          </el-descriptions-item>
        </el-descriptions>

        <h4 class="snapshot-title">变更前快照（before_snapshot）</h4>
        <pre class="snapshot-block">{{ formatJson(currentDetail.before_snapshot) }}</pre>

        <h4 class="snapshot-title">变更后快照（after_snapshot）</h4>
        <pre class="snapshot-block">{{ formatJson(currentDetail.after_snapshot) }}</pre>
      </div>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
/**
 * 审计日志查看页（P13 批 1 P3-2）
 * - 后端路由：/api/v1/erp/audit-logs（list / detail / export）
 * - 强租户隔离：由 request.ts 拦截器自动注入 X-Tenant-Id，无需前端手动传
 */
import { ref, reactive, onMounted, h } from 'vue'
import { ElMessage, ElTag, ElButton } from 'element-plus'
import { Search, Refresh, Download } from '@element-plus/icons-vue'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import {
  listAuditLogs,
  getAuditLog,
  exportAuditLogs,
  type AuditLogItem,
  type AuditLogDetail,
  type OperationType,
  type Severity,
} from '@/api/audit'

// 操作类型下拉选项（与后端 OperationType 枚举同步）
const OP_TYPE_OPTIONS: { value: OperationType; label: string }[] = [
  { value: 'CREATE', label: '新建' },
  { value: 'UPDATE', label: '更新' },
  { value: 'DELETE', label: '删除' },
  { value: 'LOGIN', label: '登录' },
  { value: 'LOGOUT', label: '登出' },
  { value: 'EXPORT', label: '导出' },
  { value: 'QUERY', label: '查询' },
  { value: 'OTHER', label: '其它' },
]

// 操作类型中文标签
const OP_TYPE_LABELS: Record<string, string> = Object.fromEntries(
  OP_TYPE_OPTIONS.map((o) => [o.value, o.label]),
)

// 操作类型对应的 el-tag 颜色
const OP_TYPE_TAG: Record<string, 'primary' | 'success' | 'warning' | 'info' | 'danger'> = {
  CREATE: 'success',
  UPDATE: 'primary',
  DELETE: 'danger',
  LOGIN: 'info',
  LOGOUT: 'info',
  EXPORT: 'warning',
  QUERY: 'info',
  OTHER: 'info',
}

// 严重级别下拉选项
const SEVERITY_OPTIONS: { value: Severity; label: string }[] = [
  { value: 'INFO', label: '信息' },
  { value: 'WARN', label: '警告' },
  { value: 'ERROR', label: '错误' },
  { value: 'CRITICAL', label: '严重' },
]

// 严重级别对应的 el-tag 颜色
const SEVERITY_TAG: Record<string, 'primary' | 'success' | 'warning' | 'info' | 'danger'> = {
  INFO: 'info',
  WARN: 'warning',
  ERROR: 'danger',
  CRITICAL: 'danger',
}

// 筛选表单
const filterForm = reactive({
  dateRange: [] as string[],
  operation_type: '' as OperationType | '',
  severity: '' as Severity | '',
  resource_type: '',
  request_id: '',
  keyword: '',
})

// 分页状态
const page = ref(1)
const pageSize = ref(20)
const total = ref(0)
const loading = ref(false)
const data = ref<AuditLogItem[]>([])

// 详情抽屉
const detailVisible = ref(false)
const detailLoading = ref(false)
const currentDetail = ref<AuditLogDetail | null>(null)

// 表格列定义
const columns: ColumnDef[] = [
  { key: 'id', title: 'ID', width: 70 },
  { key: 'created_at', title: '操作时间', width: 170, formatter: (row: Record<string, unknown>) => formatDateTime(row.created_at as string | null | undefined) },
  {
    key: 'operation_type',
    title: '操作类型',
    width: 100,
    renderCell: (row: Record<string, unknown>) =>
      h(
        ElTag,
        { type: OP_TYPE_TAG[row.operation_type as string] ?? 'info', size: 'small' },
        () => OP_TYPE_LABELS[row.operation_type as string] ?? row.operation_type ?? '-',
      ),
  },
  {
    key: 'severity',
    title: '级别',
    width: 80,
    renderCell: (row: Record<string, unknown>) =>
      h(
        ElTag,
        { type: SEVERITY_TAG[row.severity as string] ?? 'info', size: 'small' },
        () => row.severity ?? '-',
      ),
  },
  { key: 'username', title: '操作人', width: 110 },
  { key: 'resource_type', title: '资源类型', width: 110 },
  { key: 'resource_id', title: '资源 ID', width: 110 },
  { key: 'ip_address', title: '客户端 IP', width: 130 },
  { key: 'request_id', title: '请求追踪', width: 130 },
  { key: 'description', title: '描述', minWidth: 200 },
  {
    key: 'actions',
    title: '操作',
    width: 80,
    fixed: 'right',
    renderCell: (row: Record<string, unknown>) =>
      h(
        ElButton,
        {
          type: 'primary',
          size: 'small',
          link: true,
          onClick: (e: Event) => {
            e.stopPropagation()
            handleViewDetail(row as unknown as AuditLogItem)
          },
        },
        () => '详情',
      ),
  },
] as unknown as ColumnDef[]

/**
 * 列表查询（统一在此构造查询参数）
 */
const buildListParams = () => {
  const params: Record<string, unknown> = {
    page: page.value,
    page_size: pageSize.value,
  }
  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    params.start_time = filterForm.dateRange[0]
    params.end_time = filterForm.dateRange[1]
  }
  if (filterForm.operation_type) params.operation_type = filterForm.operation_type
  if (filterForm.severity) params.severity = filterForm.severity
  if (filterForm.resource_type.trim()) params.resource_type = filterForm.resource_type.trim()
  if (filterForm.request_id.trim()) params.request_id = filterForm.request_id.trim()
  if (filterForm.keyword.trim()) params.keyword = filterForm.keyword.trim()
  return params
}

/**
 * 加载列表数据
 */
const loadData = async () => {
  loading.value = true
  try {
    const res = await listAuditLogs(buildListParams())
    data.value = res.items
    total.value = res.total
  } catch (err) {
    ElMessage.error('加载审计日志失败')
  } finally {
    loading.value = false
  }
}

/**
 * 查询按钮：重置到第一页
 */
const handleQuery = () => {
  page.value = 1
  loadData()
}

/**
 * 重置筛选条件
 */
const handleReset = () => {
  filterForm.dateRange = []
  filterForm.operation_type = ''
  filterForm.severity = ''
  filterForm.resource_type = ''
  filterForm.request_id = ''
  filterForm.keyword = ''
  page.value = 1
  loadData()
}

/**
 * 分页变化
 */
const handlePageChange = (p: number) => {
  page.value = p
  loadData()
}

/**
 * 每页大小变化
 */
const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
  loadData()
}

/**
 * 行点击：打开详情
 */
const handleRowClick = (row: AuditLogItem) => {
  handleViewDetail(row)
}

/**
 * 打开详情
 */
const handleViewDetail = async (row: AuditLogItem) => {
  detailVisible.value = true
  detailLoading.value = true
  try {
    currentDetail.value = await getAuditLog(row.id)
  } catch (err) {
    ElMessage.error('加载审计日志详情失败')
    detailVisible.value = false
  } finally {
    detailLoading.value = false
  }
}

/**
 * 导出 CSV：触发浏览器下载
 */
const handleExport = async () => {
  try {
    const params: Record<string, unknown> = {}
    if (filterForm.dateRange && filterForm.dateRange.length === 2) {
      params.start_time = filterForm.dateRange[0]
      params.end_time = filterForm.dateRange[1]
    }
    if (filterForm.operation_type) params.operation_type = filterForm.operation_type
    if (filterForm.severity) params.severity = filterForm.severity
    if (filterForm.resource_type.trim()) params.resource_type = filterForm.resource_type.trim()
    if (filterForm.request_id.trim()) params.request_id = filterForm.request_id.trim()
    if (filterForm.keyword.trim()) params.keyword = filterForm.keyword.trim()

    const blob = await exportAuditLogs(params)
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    const filename = `audit_logs_${new Date().toISOString().replace(/[:.]/g, '-')}.csv`
    link.href = url
    link.download = filename
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    URL.revokeObjectURL(url)
    ElMessage.success('审计日志导出已开始下载')
  } catch (err) {
    ElMessage.error('导出审计日志失败')
  }
}

/**
 * 格式化 JSON 字段（无值时显示占位符）
 */
const formatJson = (v: unknown): string => {
  if (v === null || v === undefined || v === '') return '（无）'
  try {
    return JSON.stringify(v, null, 2)
  } catch {
    return String(v)
  }
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

onMounted(() => {
  loadData()
})
</script>

<style scoped>
.audit-log-view {
  padding: 16px;
}
.filter-card {
  margin-bottom: 16px;
}
.table-card {
  margin-bottom: 16px;
}
.detail-content {
  padding: 0 16px;
}
.snapshot-title {
  margin: 20px 0 8px;
  font-size: 14px;
  color: #303133;
  font-weight: 600;
}
.snapshot-block {
  background: #f5f7fa;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  line-height: 1.5;
  max-height: 320px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
