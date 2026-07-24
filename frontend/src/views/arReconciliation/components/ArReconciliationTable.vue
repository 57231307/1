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
      <el-table-column prop="customer_code" :label="$t('arReconciliationModule.customerCode')" width="120" />
      <el-table-column prop="customer_name" :label="$t('arReconciliationModule.customerName')" width="160" />
      <el-table-column :label="$t('arReconciliationModule.matchStatus')" width="100">
        <template #default="scope">
          <el-tag :type="getMatchType(scope.row.match_status)" size="small">
            {{ $t(getMatchLabel(scope.row.match_status)) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="invoice_amount" :label="$t('arReconciliationModule.invoiceAmount')" width="130" align="right">
        <template #default="scope">{{ scope.row.invoice_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="payment_amount" :label="$t('arReconciliationModule.paymentAmount')" width="130" align="right">
        <template #default="scope">{{ scope.row.payment_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="difference" :label="$t('arReconciliationModule.differenceAmount')" width="130" align="right">
        <template #default="scope">
          <span :style="{ color: scope.row.difference !== 0 ? '#f56c6c' : '#67c23a' }">
            {{ scope.row.difference.toFixed(2) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column prop="matched_count" :label="$t('arReconciliationModule.matchedCount')" width="80" align="center" />
      <el-table-column prop="unmatched_count" :label="$t('arReconciliationModule.unmatchedCount')" width="80" align="center" />
      <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
      <el-table-column :label="$t('common.operation')" width="240" align="center">
        <template #default="scope">
          <el-button size="small" @click="emit('view-detail', scope.row as AutoReconciliationResult)">
            <el-icon><View /></el-icon> {{ $t('common.detail') }}
          </el-button>
          <el-button
            size="small"
            type="primary"
            @click="emit('send-confirmation', scope.row as AutoReconciliationResult)"
          >
            <el-icon><Promotion /></el-icon> {{ $t('common.confirm') }}
          </el-button>
          <el-button size="small" type="danger" @click="emit('open-dispute', scope.row as AutoReconciliationResult)">
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
import { useI18n } from 'vue-i18n'
import { View, Promotion, CircleClose } from '@element-plus/icons-vue'
import type { AutoReconciliationResult } from '@/api/ar-reconciliation-enhanced'
import { getMatchLabel, getMatchType } from '../composables/arRecFmts'

const { t } = useI18n({ useScope: 'global' })
void t

interface ArPagination {
  page: number
  pageSize: number
}

const props = defineProps<{
  data: AutoReconciliationResult[]
  loading: boolean
  total: number
  pagination: ArPagination
}>()

const emit = defineEmits<{
  'view-detail': [row: AutoReconciliationResult]
  'send-confirmation': [row: AutoReconciliationResult]
  'open-dispute': [row: AutoReconciliationResult]
  'page-change': [page: number]
  'page-size-change': [size: number]
}>()

void props
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
