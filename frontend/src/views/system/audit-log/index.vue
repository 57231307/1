<!--
  AuditLogView.vue - 审计日志查看页（P13 批 1 P3-2）
  任务编号: P13 批 1 H
  关联 spec: docs/superpowers/plans/2026-06-18-p13-batch1-comprehensive-plan.md §2.1
  功能：
  - 分页 + 多维筛选（时间范围 / 用户 / 操作类型 / 严重级别 / 资源类型 / 关键字）
  - V2Table 列表展示
  - 详情抽屉（el-drawer）展示 before/after 差异 JSON
  - CSV 导出按钮
  D05 Batch 4：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="audit-log-view">
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="filterForm"
        :aria-label="$t('auditLog.filter.ariaLabel')"
        @submit.prevent="handleQuery"
      >
        <el-form-item :label="$t('auditLog.filter.dateRange')">
          <el-date-picker
            v-model="filterForm.dateRange"
            type="datetimerange"
            range-separator="—"
            :start-placeholder="$t('auditLog.filter.startPlaceholder')"
            :end-placeholder="$t('auditLog.filter.endPlaceholder')"
            value-format="YYYY-MM-DDTHH:mm:ss[Z]"
            style="width: 360px"
          />
        </el-form-item>
        <el-form-item :label="$t('auditLog.filter.operationType')">
          <el-select
            v-model="filterForm.operation_type"
            :placeholder="$t('auditLog.filter.all')"
            clearable
            style="width: 160px"
          >
            <el-option
              v-for="opt in opTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('auditLog.filter.severity')">
          <el-select
            v-model="filterForm.severity"
            :placeholder="$t('auditLog.filter.all')"
            clearable
            style="width: 140px"
          >
            <el-option
              v-for="opt in severityOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('auditLog.filter.resourceType')">
          <el-input
            v-model="filterForm.resource_type"
            :placeholder="$t('auditLog.filter.resourceTypePlaceholder')"
            clearable
            style="width: 160px"
          />
        </el-form-item>
        <el-form-item :label="$t('auditLog.filter.requestId')">
          <el-input
            v-model="filterForm.request_id"
            placeholder="trace_id"
            clearable
            style="width: 200px"
          />
        </el-form-item>
        <el-form-item :label="$t('auditLog.filter.keyword')">
          <el-input
            v-model="filterForm.keyword"
            :placeholder="$t('auditLog.filter.keywordPlaceholder')"
            clearable
            style="width: 180px"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            {{ $t('auditLog.filter.query') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            {{ $t('auditLog.filter.reset') }}
          </el-button>
          <el-button
            v-permission="EXPORT_APPROVAL_REQUEST_PERMISSION_KEY"
            :loading="exportSubmitting"
            type="success"
            @click="handleExport"
          >
            <el-icon><Download /></el-icon>
            {{ $t('auditLog.filter.exportCsv') }}
          </el-button>
          <!-- 就地下载本人在本资源下已批准的导出（令牌随申请落库，批准后回到本页下载） -->
          <el-button
            v-permission="EXPORT_APPROVAL_REQUEST_PERMISSION_KEY"
            :loading="approvedDownloading"
            @click="handleDownloadMine"
          >
            <el-icon><Download /></el-icon>
            {{ $t('exportApprovals.downloadMine') }}
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
        :empty-text="$t('auditLog.table.emptyText')"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
        @row-click="handleRowClick"
      />
    </el-card>

    <!-- 详情抽屉：展示 before/after 差异快照 -->
    <el-drawer
      v-model="detailVisible"
      :title="$t('auditLog.detail.title')"
      size="60%"
      direction="rtl"
      :destroy-on-close="true"
    >
      <div v-if="currentDetail" class="detail-content">
        <el-descriptions :column="2" border>
          <el-descriptions-item :label="$t('auditLog.detail.logId')">{{
            currentDetail.id
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.operator')">
            {{ currentDetail.username ?? '-' }} (#{{ currentDetail.user_id ?? '-' }})
          </el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.operationTime')">
            {{ formatDateTime(currentDetail.created_at) }}
          </el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.operationType')">
            <el-tag :type="OP_TYPE_TAG[currentDetail.operation_type ?? ''] ?? 'info'" size="small">
              {{ getOpTypeLabel(currentDetail.operation_type ?? '') }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.severity')">
            <el-tag :type="SEVERITY_TAG[currentDetail.severity ?? ''] ?? 'info'" size="small">
              {{ getSeverityLabel(currentDetail.severity ?? '') }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.resourceType')">{{
            currentDetail.resource_type ?? '-'
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.resourceId')">{{
            currentDetail.resource_id ?? '-'
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.resourceName')">{{
            currentDetail.resource_name ?? '-'
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.requestMethod')">{{
            currentDetail.request_method ?? '-'
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.requestPath')" :span="2">
            {{ currentDetail.request_path ?? '-' }}
          </el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.requestTrace')">{{
            currentDetail.request_id ?? '-'
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.clientIp')">{{
            currentDetail.ip_address ?? '-'
          }}</el-descriptions-item>
          <el-descriptions-item label="User-Agent" :span="2">
            <el-text line-clamp="2">{{ currentDetail.user_agent ?? '-' }}</el-text>
          </el-descriptions-item>
          <el-descriptions-item :label="$t('auditLog.detail.description')" :span="2">
            {{ currentDetail.description ?? '-' }}
          </el-descriptions-item>
        </el-descriptions>

        <h4 class="snapshot-title">{{ $t('auditLog.detail.beforeSnapshot') }}</h4>
        <pre class="snapshot-block">{{ formatJson(currentDetail.before_snapshot) }}</pre>

        <h4 class="snapshot-title">{{ $t('auditLog.detail.afterSnapshot') }}</h4>
        <pre class="snapshot-block">{{ formatJson(currentDetail.after_snapshot) }}</pre>
      </div>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
/**
 * 审计日志查看页（P13 批 1 P3-2）
 * - 后端路由：/api/v1/erp/audit-logs（list / detail / export）
 */
import { ref, reactive, computed, h } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElTag, ElButton } from 'element-plus';
import { Search, Refresh, Download } from '@element-plus/icons-vue';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import { useTableApi } from '@/composables/useTableApi';
import {
  getAuditLog,
  type AuditLogItem,
  type AuditLogDetail,
  type OperationType,
  type Severity,
} from '@/api/audit';
// 审计日志属敏感导出：后端 GET /audit-logs/export 以 download_token 做 fail-closed
// 审批令牌校验（admin 权限 + xlsx + 自审计 + 水印），导出入口改为提交审批申请
import { EXPORT_APPROVAL_REQUEST_PERMISSION_KEY } from '@/constants/export-approval';
import { useExportApproval } from '@/composables/useExportApproval';

const { t } = useI18n({ useScope: 'global' });

// D05 Batch 4：操作类型/严重级别下拉选项改为 computed，使 t() 在语言切换时响应式求值
const opTypeOptions = computed<{ value: OperationType; label: string }[]>(() => [
  { value: 'CREATE', label: t('auditLog.operationType.create') },
  { value: 'UPDATE', label: t('auditLog.operationType.update') },
  { value: 'DELETE', label: t('auditLog.operationType.delete') },
  { value: 'LOGIN', label: t('auditLog.operationType.login') },
  { value: 'LOGOUT', label: t('auditLog.operationType.logout') },
  { value: 'EXPORT', label: t('auditLog.operationType.export') },
  { value: 'QUERY', label: t('auditLog.operationType.query') },
  { value: 'OTHER', label: t('auditLog.operationType.other') },
]);

// D05 Batch 4：操作类型标签改为函数，使 t() 在每次渲染时响应式求值
const getOpTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    CREATE: t('auditLog.operationType.create'),
    UPDATE: t('auditLog.operationType.update'),
    DELETE: t('auditLog.operationType.delete'),
    LOGIN: t('auditLog.operationType.login'),
    LOGOUT: t('auditLog.operationType.logout'),
    EXPORT: t('auditLog.operationType.export'),
    QUERY: t('auditLog.operationType.query'),
    OTHER: t('auditLog.operationType.other'),
  };
  return labels[type] || type;
};

const severityOptions = computed<{ value: Severity; label: string }[]>(() => [
  { value: 'INFO', label: t('auditLog.severityLevel.info') },
  { value: 'WARN', label: t('auditLog.severityLevel.warn') },
  { value: 'ERROR', label: t('auditLog.severityLevel.error') },
  { value: 'CRITICAL', label: t('auditLog.severityLevel.critical') },
]);

// D05 Batch 4：严重级别标签改为函数
const getSeverityLabel = (severity: string) => {
  const labels: Record<string, string> = {
    INFO: t('auditLog.severityLevel.info'),
    WARN: t('auditLog.severityLevel.warn'),
    ERROR: t('auditLog.severityLevel.error'),
    CRITICAL: t('auditLog.severityLevel.critical'),
  };
  return labels[severity] || severity;
};

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
};

// 严重级别对应的 el-tag 颜色
const SEVERITY_TAG: Record<string, 'primary' | 'success' | 'warning' | 'info' | 'danger'> = {
  INFO: 'info',
  WARN: 'warning',
  ERROR: 'danger',
  CRITICAL: 'danger',
};

// 筛选表单
const filterForm = reactive({
  dateRange: [] as string[],
  operation_type: '' as OperationType | '',
  severity: '' as Severity | '',
  resource_type: '',
  request_id: '',
  keyword: '',
});

// 批次 267：接入 useTableApi，消除手写 page/pageSize/total/loading + loadData 重复
// API 返回 { items, total }，配置 listKey: 'items' 让 useTableApi 正确探测
const { data, loading, page, pageSize, total, refresh, setQueryParam } = useTableApi<AuditLogItem>({
  url: '/audit-logs',
  listKey: 'items',
  onError: () => ElMessage.error(t('auditLog.message.loadFailed')),
});

// 详情抽屉
const detailVisible = ref(false);
const detailLoading = ref(false);
const currentDetail = ref<AuditLogDetail | null>(null);

// D05 Batch 4：表格列定义改为 computed，使 title 在语言切换时响应式求值
const columns = computed<ColumnDef<AuditLogItem>[]>(() => [
  { key: 'id', title: 'ID', width: 70 },
  {
    key: 'created_at',
    title: t('auditLog.table.operationTime'),
    width: 170,
    formatter: row => formatDateTime(row.created_at),
  },
  {
    key: 'operation_type',
    title: t('auditLog.table.operationType'),
    width: 100,
    renderCell: row =>
      h(ElTag, { type: OP_TYPE_TAG[row.operation_type ?? ''] ?? 'info', size: 'small' }, () =>
        getOpTypeLabel(row.operation_type ?? '')
      ),
  },
  {
    key: 'severity',
    title: t('auditLog.table.severity'),
    width: 80,
    renderCell: row =>
      h(ElTag, { type: SEVERITY_TAG[row.severity ?? ''] ?? 'info', size: 'small' }, () =>
        getSeverityLabel(row.severity ?? '')
      ),
  },
  { key: 'username', title: t('auditLog.table.operator'), width: 110 },
  { key: 'resource_type', title: t('auditLog.table.resourceType'), width: 110 },
  { key: 'resource_id', title: t('auditLog.table.resourceId'), width: 110 },
  { key: 'ip_address', title: t('auditLog.table.clientIp'), width: 130 },
  { key: 'request_id', title: t('auditLog.table.requestTrace'), width: 130 },
  { key: 'description', title: t('auditLog.table.description'), minWidth: 200 },
  {
    key: 'actions',
    title: t('auditLog.table.operation'),
    width: 80,
    fixed: 'right',
    renderCell: row =>
      h(
        ElButton,
        {
          type: 'primary',
          size: 'small',
          link: true,
          onClick: (e: Event) => {
            e.stopPropagation();
            handleViewDetail(row);
          },
        },
        () => t('auditLog.table.detail')
      ),
  },
]);

/**
 * 批次 267：同步筛选条件到 useTableApi.queryParams 并刷新
 * useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
 */
const syncQueryParams = () => {
  // 先清空旧筛选，再写入新值（避免上次筛选残留）
  setQueryParam('start_time', undefined);
  setQueryParam('end_time', undefined);
  setQueryParam('operation_type', undefined);
  setQueryParam('severity', undefined);
  setQueryParam('resource_type', undefined);
  setQueryParam('request_id', undefined);
  setQueryParam('keyword', undefined);

  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    setQueryParam('start_time', filterForm.dateRange[0]);
    setQueryParam('end_time', filterForm.dateRange[1]);
  }
  if (filterForm.operation_type) setQueryParam('operation_type', filterForm.operation_type);
  if (filterForm.severity) setQueryParam('severity', filterForm.severity);
  if (filterForm.resource_type.trim())
    setQueryParam('resource_type', filterForm.resource_type.trim());
  if (filterForm.request_id.trim()) setQueryParam('request_id', filterForm.request_id.trim());
  if (filterForm.keyword.trim()) setQueryParam('keyword', filterForm.keyword.trim());
};

/**
 * 查询按钮：同步筛选 + 重置到第一页 + 刷新
 */
const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  refresh();
};

/**
 * 重置筛选条件
 */
const handleReset = () => {
  filterForm.dateRange = [];
  filterForm.operation_type = '';
  filterForm.severity = '';
  filterForm.resource_type = '';
  filterForm.request_id = '';
  filterForm.keyword = '';
  syncQueryParams();
  page.value = 1;
  refresh();
};

/**
 * 分页变化（useTableApi 自动 watch 重载，此处仅更新 page 值）
 */
const handlePageChange = (p: number) => {
  page.value = p;
};

/**
 * 每页大小变化（useTableApi 自动 watch 重载，切回第一页）
 */
const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

/**
 * 行点击：打开详情
 */
const handleRowClick = (row: AuditLogItem) => {
  handleViewDetail(row);
};

/**
 * 打开详情
 */
const handleViewDetail = async (row: AuditLogItem) => {
  detailVisible.value = true;
  detailLoading.value = true;
  try {
    currentDetail.value = await getAuditLog(row.id);
  } catch (err) {
    ElMessage.error(t('auditLog.message.loadDetailFailed'));
    detailVisible.value = false;
  } finally {
    detailLoading.value = false;
  }
};

/**
 * 导出审计日志：改为提交导出审批申请（useExportApproval 统一入口，参数形状由
 * constants/export-approval 注册表收敛），批准后由「下载我的导出」按钮在本页就地下载
 * 带水印 xlsx（downloadApprovedExport 拉取本人该资源下可用令牌申请，不再集中到审批页
 * 下载）；后端下载端点自带导出行为二次审计闭环。
 * 参数与 syncQueryParams 对齐：dateRange → start_time/end_time +
 * operation_type/severity/resource_type/request_id/keyword
 */
const { exportSubmitting, requestExport, approvedDownloading, downloadApprovedExport } =
  useExportApproval();
const handleExport = () => {
  const params: Record<string, unknown> = {
    operation_type: filterForm.operation_type,
    severity: filterForm.severity,
    resource_type: filterForm.resource_type.trim(),
    request_id: filterForm.request_id.trim(),
    keyword: filterForm.keyword.trim(),
  };
  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    params.start_time = filterForm.dateRange[0];
    params.end_time = filterForm.dateRange[1];
  }
  return requestExport('audit_log', params, total.value);
};
const handleDownloadMine = () => downloadApprovedExport('audit_log');

/**
 * 格式化 JSON 字段（无值时显示占位符）
 */
const formatJson = (v: unknown): string => {
  if (v === null || v === undefined || v === '') return t('auditLog.detail.emptyValue');
  try {
    return JSON.stringify(v, null, 2);
  } catch {
    return String(v);
  }
};

/**
 * 格式化日期时间
 */
const formatDateTime = (v: string | null | undefined): string => {
  if (!v) return '-';
  const d = new Date(v);
  if (isNaN(d.getTime())) return v;
  const pad = (n: number) => n.toString().padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
};

// 批次 267：useTableApi 构造时自动初始加载，无需 onMounted
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
