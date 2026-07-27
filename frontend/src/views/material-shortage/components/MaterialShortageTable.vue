<!--
  MaterialShortageTable.vue - 物料短缺列表（含过滤栏、操作按钮）
  拆分自 material-shortage/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-bar">
      <el-select
        :model-value="filterSeverity"
        :placeholder="t('materialShortage.table.severityPlaceholder')"
        clearable
        style="width: 160px"
        @update:model-value="(v: string) => emit('update:filter-severity', v)"
      >
        <el-option :label="t('materialShortage.table.severityCritical')" value="critical" />
        <el-option :label="t('materialShortage.table.severityHigh')" value="high" />
        <el-option :label="t('materialShortage.table.severityMedium')" value="medium" />
        <el-option :label="t('materialShortage.table.severityLow')" value="low" />
      </el-select>
      <el-select
        :model-value="filterStatus"
        :placeholder="t('materialShortage.table.statusPlaceholder')"
        clearable
        style="width: 140px"
        @update:model-value="(v: string) => emit('update:filter-status', v)"
      >
        <el-option :label="t('materialShortage.table.statusPending')" value="pending" />
        <el-option :label="t('materialShortage.table.statusNotified')" value="notified" />
        <el-option :label="t('materialShortage.table.statusResolved')" value="resolved" />
      </el-select>
      <el-button type="primary" @click="emit('filter-change')">
        <el-icon><Search /></el-icon>
        {{ t('materialShortage.table.search') }}
      </el-button>
      <el-button type="success" :loading="checking" @click="emit('check')">
        <el-icon><Refresh /></el-icon>
        {{ t('materialShortage.table.triggerCheck') }}
      </el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="data"
      stripe
      :aria-label="t('materialShortage.table.ariaLabel')"
    >
      <el-table-column
        prop="material_code"
        :label="t('materialShortage.table.materialCode')"
        min-width="140"
      />
      <el-table-column
        prop="material_name"
        :label="t('materialShortage.table.materialName')"
        min-width="160"
      />
      <el-table-column
        prop="shortage_quantity"
        :label="t('materialShortage.table.shortageQuantity')"
        width="100"
        align="right"
      />
      <el-table-column
        prop="required_quantity"
        :label="t('materialShortage.table.requiredQuantity')"
        width="100"
        align="right"
      />
      <el-table-column
        prop="available_quantity"
        :label="t('materialShortage.table.availableQuantity')"
        width="100"
        align="right"
      />
      <el-table-column
        prop="severity"
        :label="t('materialShortage.table.severity')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getSeverityColor(row.severity)">
            {{ getSeverityLabel(row.severity) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="status"
        :label="t('materialShortage.table.status')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getStatusColor(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="source_type"
        :label="t('materialShortage.table.sourceType')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getSourceTypeColor(row.source_type)">
            {{ getSourceTypeLabel(row.source_type) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="source_no"
        :label="t('materialShortage.table.sourceNo')"
        min-width="140"
      />
      <el-table-column
        prop="expected_arrival_date"
        :label="t('materialShortage.table.expectedArrival')"
        min-width="120"
      />
      <el-table-column
        prop="remark"
        :label="t('materialShortage.table.remark')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column :label="t('materialShortage.table.operation')" width="180" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'pending'"
            type="primary"
            link
            size="small"
            @click="emit('notify', row)"
          >
            {{ t('materialShortage.table.sendNotify') }}
          </el-button>
          <el-button
            v-if="row.status !== 'resolved'"
            type="success"
            link
            size="small"
            @click="emit('resolve', row)"
          >
            {{ t('materialShortage.table.markResolve') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="currentPage"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('materialShortage.table.paginationAriaLabel')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Search, Refresh } from '@element-plus/icons-vue';
import { getSeverityColor, getStatusColor, getSourceTypeColor } from '../composables/msFmts';
import type { MaterialShortage } from '@/api/material-shortage';

const { t } = useI18n({ useScope: 'global' });

/**
 * 列表组件（含过滤栏 + 操作）
 */
defineProps<{
  // 列表数据
  data: MaterialShortage[];
  // 总数
  total: number;
  // 加载状态
  loading: boolean;
  // 检查中
  checking: boolean;
  // 分页
  currentPage: number;
  pageSize: number;
  // 过滤
  filterSeverity: string;
  filterStatus: string;
}>();

const emit = defineEmits<{
  // 过滤变化
  'filter-change': [];
  // 触发检查
  check: [];
  // 通知
  notify: [row: MaterialShortage];
  // 解决
  resolve: [row: MaterialShortage];
  // 分页
  'update:page': [v: number];
  'update:size': [v: number];
  // 过滤值变化
  'update:filter-severity': [v: string];
  'update:filter-status': [v: string];
}>();

/**
 * 严重程度标签映射（基于 i18n）
 */
const getSeverityLabel = (severity: string) => {
  const map: Record<string, string> = {
    critical: t('materialShortage.table.severityCritical'),
    high: t('materialShortage.table.severityHigh'),
    medium: t('materialShortage.table.severityMedium'),
    low: t('materialShortage.table.severityLow'),
  };
  return map[severity] || severity;
};

/**
 * 状态标签映射（基于 i18n）
 */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('materialShortage.table.statusPending'),
    notified: t('materialShortage.table.statusNotified'),
    resolved: t('materialShortage.table.statusResolved'),
  };
  return map[status] || status;
};

/**
 * 来源类型标签映射（基于 i18n）
 */
const getSourceTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    production: t('materialShortage.table.sourceProduction'),
    sales: t('materialShortage.table.sourceSales'),
    purchase: t('materialShortage.table.sourcePurchase'),
  };
  return map[type] || type;
};
</script>

<style scoped>
.filter-bar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
  align-items: center;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
