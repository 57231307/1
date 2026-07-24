<!--
  ArReconciliationTable.vue - AR 对账结果列表
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>对账结果列表</span>
        <el-tag type="info">共 {{ total }} 条</el-tag>
      </div>
    </template>
    <el-table
      :data="data"
      :loading="loading"
      border
      fit
      highlight-current-row
      style="width: 100%"
      aria-label="AR 对账结果列表"
    >
      <el-table-column prop="customer_code" label="客户编码" width="120" />
      <el-table-column prop="customer_name" label="客户名称" width="160" />
      <el-table-column label="匹配状态" width="100">
        <template #default="scope">
          <el-tag :type="getMatchType(scope.row.match_status)" size="small">
            {{ getMatchLabel(scope.row.match_status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="invoice_amount" label="发票金额" width="130" align="right">
        <template #default="scope">{{ scope.row.invoice_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="payment_amount" label="回款金额" width="130" align="right">
        <template #default="scope">{{ scope.row.payment_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="difference" label="差异金额" width="130" align="right">
        <template #default="scope">
          <span :style="{ color: scope.row.difference !== 0 ? '#f56c6c' : '#67c23a' }">
            {{ scope.row.difference.toFixed(2) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column prop="matched_count" label="已匹配" width="80" align="center" />
      <el-table-column prop="unmatched_count" label="未匹配" width="80" align="center" />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="240" align="center">
        <template #default="scope">
          <el-button size="small" @click="emit('view-detail', scope.row as AutoReconciliationResult)">
            <el-icon><View /></el-icon> 明细
          </el-button>
          <el-button
            size="small"
            type="primary"
            @click="emit('send-confirmation', scope.row as AutoReconciliationResult)"
          >
            <el-icon><Promotion /></el-icon> 确认
          </el-button>
          <el-button size="small" type="danger" @click="emit('open-dispute', scope.row as AutoReconciliationResult)">
            <el-icon><CircleClose /></el-icon> 争议
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
      aria-label="AR 对账结果分页"
      @current-change="(p: number) => emit('page-change', p)"
      @size-change="(s: number) => emit('page-size-change', s)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { View, Promotion, CircleClose } from '@element-plus/icons-vue'
import type { AutoReconciliationResult } from '@/api/ar-reconciliation-enhanced'
import { getMatchLabel, getMatchType } from '../composables/arRecFmts'

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
