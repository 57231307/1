<!--
  ArReconciliationTable.vue - AR 对账结果列表
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>{{ $t('arReconciliationModule.resultListTitle') }}</span>
        <el-tag type="info">{{ $t('common.total') }} {{ total }} {{ $t('common.items') }}</el-tag>
      </div>
    </template>
    <el-table
      :data="data"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
      :aria-label="$t('arReconciliationModule.resultListAria')"
    >
      <el-table-column
        prop="reconciliation_no"
        :label="$t('arReconciliationModule.index.reconciliationNo')"
        width="150"
      />
      <el-table-column
        prop="customer_name"
        :label="$t('arReconciliationModule.index.customerName')"
        width="160"
      >
        <template #default="scope">{{ scope.row.customer_name || '-' }}</template>
      </el-table-column>
      <el-table-column
        prop="period_start"
        :label="$t('arReconciliationModule.index.reconciliationStart')"
        width="120"
      />
      <el-table-column
        prop="period_end"
        :label="$t('arReconciliationModule.index.reconciliationEnd')"
        width="120"
      />
      <el-table-column
        prop="total_invoices"
        :label="$t('arReconciliationModule.index.invoiceAmount')"
        width="130"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.total_invoices) }}</template>
      </el-table-column>
      <el-table-column
        prop="total_collections"
        :label="$t('arReconciliationModule.index.paymentAmount')"
        width="130"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.total_collections) }}</template>
      </el-table-column>
      <el-table-column
        prop="closing_balance"
        :label="$t('arReconciliationModule.index.balance')"
        width="130"
        align="right"
      >
        <template #default="scope">
          <span
            :style="{ color: Number(scope.row.closing_balance ?? 0) !== 0 ? '#f56c6c' : '#67c23a' }"
          >
            {{ fmtNum(scope.row.closing_balance) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column :label="$t('common.status')" width="110" align="center">
        <template #default="scope">
          <el-tag
            :type="scope.row.reconciliation_status === 'confirmed' ? 'success' : 'info'"
            size="small"
          >
            {{ statusLabel(scope.row.reconciliation_status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
      <el-table-column :label="$t('common.operation')" width="240" align="center">
        <template #default="scope">
          <el-button
            size="small"
            @click="emit('view-detail', scope.row as AutoReconciliationResult)"
          >
            <el-icon><View /></el-icon> {{ $t('common.detail') }}
          </el-button>
          <el-button
            size="small"
            type="primary"
            @click="emit('send-confirmation', scope.row as AutoReconciliationResult)"
          >
            <el-icon><Promotion /></el-icon> {{ $t('common.confirm') }}
          </el-button>
          <el-button
            size="small"
            type="danger"
            @click="emit('open-dispute', scope.row as AutoReconciliationResult)"
          >
            <el-icon><CircleClose /></el-icon> {{ $t('arReconciliationModule.dispute') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      :current-page="pagination.page"
      :page-size="pagination.pageSize"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next"
      class="pagination-container"
      :aria-label="$t('arReconciliationModule.resultPaginationAria')"
      @current-change="(p: number) => emit('page-change', p)"
      @size-change="(s: number) => emit('page-size-change', s)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { View, Promotion, CircleClose } from '@element-plus/icons-vue';
import type { AutoReconciliationResult } from '@/api/ar-reconciliation-enhanced';

const { t } = useI18n({ useScope: 'global' });

// 后端 Decimal 序列化为字符串，统一转数字再格式化，避免直接 toFixed 崩溃
const fmtNum = (v: unknown): string => Number(v ?? 0).toFixed(2);

// 对账状态词表写入方为后端 models/status/finance.rs 的 ar 段，
// 展示词键当前只覆盖 draft 与 confirmed，其余状态回显原值
const statusLabel = (v: string | null | undefined): string => {
  if (!v) return '-';
  if (v === 'draft') return t('arReconciliationModule.index.statusDraft');
  if (v === 'confirmed') return t('arReconciliationModule.index.statusConfirmed');
  return v;
};

interface ArPagination {
  page: number;
  pageSize: number;
}

const props = defineProps<{
  data: AutoReconciliationResult[];
  loading: boolean;
  total: number;
  pagination: ArPagination;
}>();

const emit = defineEmits<{
  'view-detail': [row: AutoReconciliationResult];
  'send-confirmation': [row: AutoReconciliationResult];
  'open-dispute': [row: AutoReconciliationResult];
  'page-change': [page: number];
  'page-size-change': [size: number];
}>();

void props;
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
