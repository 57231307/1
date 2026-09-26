<!--
  AuditTab.vue - 审计日志 Tab
  来源：原 system/index.vue 中 审计日志 tab 内容
  拆分日期：2026-06-15 B3-1
  批次 281：接入 useTableApi，移除手写分页 + 直接 request 调用
-->
<template>
  <div class="audit-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.audit.title') }}</h2>
    </div>
    <el-card shadow="hover">
      <el-form
        :inline="true"
        :model="filterForm"
        class="mb-4"
        :aria-label="t('system.audit.aria.filterForm')"
      >
        <el-form-item :label="t('system.audit.label.operator')">
          <el-input
            v-model="filterForm.operator"
            :placeholder="t('system.audit.placeholder.operator')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('system.audit.label.module')">
          <el-input
            v-model="filterForm.module"
            :placeholder="t('system.audit.placeholder.module')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('system.audit.label.timeRange')">
          <el-date-picker
            v-model="filterForm.dateRange"
            type="daterange"
            :range-separator="t('system.audit.common.to')"
            :start-placeholder="t('system.audit.placeholder.startDate')"
            :end-placeholder="t('system.audit.placeholder.endDate')"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            t('system.audit.button.query')
          }}</el-button>
        </el-form-item>
      </el-form>
      <el-table
        v-loading="loading"
        :data="auditLogs"
        stripe
        :aria-label="t('system.audit.aria.list')"
      >
        <el-table-column prop="created_at" :label="t('system.audit.column.time')" width="180" />
        <el-table-column prop="username" :label="t('system.audit.column.operator')" width="120" />
        <el-table-column
          prop="resource_type"
          :label="t('system.audit.column.module')"
          width="120"
        />
        <el-table-column
          prop="operation_type"
          :label="t('system.audit.column.action')"
          width="100"
        />
        <el-table-column
          prop="resource_name"
          :label="t('system.audit.column.resource')"
          width="120"
        />
        <el-table-column prop="ip_address" :label="t('system.audit.column.ip')" width="130" />
        <el-table-column
          prop="description"
          :label="t('system.audit.column.detail')"
          min-width="200"
          show-overflow-tooltip
        />
      </el-table>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        class="mt-4"
        :aria-label="t('system.audit.aria.pagination')"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { useTableApi } from '@/composables/useTableApi';
import type { AuditLogItem } from '@/api/audit';

const { t } = useI18n({ useScope: 'global' });

// filterForm 仅保留筛选字段，分页字段由 useTableApi 管理。
// operator 字段为操作人姓名筛选；/audit-logs 端点仅暴露 user_id(数字)/resource_type/
// operation_type/severity/keyword/start_time/end_time 过滤入参，无「按操作人姓名」过滤，
// 故 operator 暂不下发到 query（列为待决策，需后端补 username 过滤入参后再接线），
// 避免发送后端不识别的死参数造成「筛选恒无效」的假接线。
const filterForm = reactive({
  operator: '',
  module: '',
  dateRange: null as [string, string] | null,
});

const {
  data: auditLogs,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchAuditLogs,
  setQueryParam,
} = useTableApi<AuditLogItem>({
  // 后端真实端点：/api/v1/erp/audit-logs（system.rs::audit_logs()，admin 域）；
  // 与 api/audit.ts::getAuditLogList、system/audit-log 页同口径，返回 {data:{items,total,page,page_size}}。
  url: '/audit-logs',
  listKey: 'items',
  defaultPageSize: 20,
  // 保持原行为：查询失败不额外弹出错误（列表已能正常 200 加载，该回调仅兜底极端网络故障）
  onError: () => {},
});

const syncQueryParams = () => {
  // module 对应后端 resource_type 过滤；时间范围对应 start_time/end_time，
  // 后端以 DateTime<Utc>(RFC3339) 解析，日期粒度补足到当日首/末秒避免漏记录。
  setQueryParam('resource_type', filterForm.module || undefined);
  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    setQueryParam('start_time', `${filterForm.dateRange[0]}T00:00:00Z`);
    setQueryParam('end_time', `${filterForm.dateRange[1]}T23:59:59Z`);
  } else {
    setQueryParam('start_time', undefined);
    setQueryParam('end_time', undefined);
  }
};

const handleSearch = () => {
  syncQueryParams();
  page.value = 1;
  fetchAuditLogs().catch(e => console.error('[AuditTab] 审计日志加载失败:', e));
};

defineExpose({ refresh: fetchAuditLogs });
</script>
