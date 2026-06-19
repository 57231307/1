<!--
  ArDetail.vue - AR 对账明细对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="对账明细"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="currentReconciliation" class="detail-header">
      <el-descriptions :column="4" border>
        <el-descriptions-item label="客户编码">{{
          currentReconciliation.customer_code
        }}</el-descriptions-item>
        <el-descriptions-item label="客户名称">{{
          currentReconciliation.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item label="发票金额">{{
          currentReconciliation.invoice_amount.toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item label="回款金额">{{
          currentReconciliation.payment_amount.toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item label="差异金额">{{
          currentReconciliation.difference.toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item label="匹配状态">
          <el-tag :type="getMatchType(currentReconciliation.match_status)" size="small">
            {{ getMatchLabel(currentReconciliation.match_status) }}
          </el-tag>
        </el-descriptions-item>
      </el-descriptions>
    </div>
    <el-table :data="detailData" border style="width: 100%; margin-top: 16px">
      <el-table-column prop="type" label="类型" width="100">
        <template #default="scope">
          <el-tag
            size="small"
            :type="
              scope.row.type === 'invoice'
                ? ''
                : scope.row.type === 'payment'
                  ? 'success'
                  : 'warning'
            "
          >
            {{
              scope.row.type === 'invoice'
                ? '发票'
                : scope.row.type === 'payment'
                  ? '回款'
                  : '调整'
            }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="source_no" label="单据号" width="150" />
      <el-table-column prop="source_date" label="日期" width="120" />
      <el-table-column prop="amount" label="金额" width="120" align="right">
        <template #default="scope">{{ scope.row.amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="matched_amount" label="已匹配金额" width="120" align="right">
        <template #default="scope">{{ scope.row.matched_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="unmatched_amount" label="未匹配金额" width="120" align="right">
        <template #default="scope">{{ scope.row.unmatched_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column label="状态" width="100">
        <template #default="scope">
          <el-tag size="small" :type="getMatchType(scope.row.status)">
            {{ getMatchLabel(scope.row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="remark" label="备注" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type {
  AutoReconciliationResult,
  ReconciliationDetailItem,
} from '@/api/ar-reconciliation-enhanced'
import { getMatchLabel, getMatchType } from '../composables/arRecFmts'

/**
 * 对账明细对话框组件
 */
const props = defineProps<{
  visible: boolean
  currentReconciliation: AutoReconciliationResult | null
  detailData: ReconciliationDetailItem[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

void props
</script>

<style scoped>
.detail-header {
  margin-bottom: 16px;
}
</style>
