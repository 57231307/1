<!--
  ArReconciliationDetail.vue - AR 对账明细对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="$t('arReconciliationModule.detailTitle')"
    width="900px"
    :aria-label="$t('arReconciliationModule.detailDialogAria')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="currentReconciliation" class="detail-header">
      <el-descriptions :column="4" border>
        <el-descriptions-item :label="$t('arReconciliationModule.customerCode')">{{
          currentReconciliation.customer_code
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('arReconciliationModule.customerName')">{{
          currentReconciliation.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('arReconciliationModule.invoiceAmount')">{{
          currentReconciliation.invoice_amount.toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('arReconciliationModule.paymentAmount')">{{
          currentReconciliation.payment_amount.toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('arReconciliationModule.differenceAmount')">{{
          currentReconciliation.difference.toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item :label="$t('arReconciliationModule.matchStatus')">
          <el-tag :type="getMatchType(currentReconciliation.match_status)" size="small">
            {{ $t(getMatchLabel(currentReconciliation.match_status)) }}
          </el-tag>
        </el-descriptions-item>
      </el-descriptions>
    </div>
    <el-table
      :data="detailData"
      border
      style="width: 100%; margin-top: 16px"
      :aria-label="$t('arReconciliationModule.detailTableAria')"
    >
      <el-table-column prop="item_type" :label="$t('arReconciliationModule.type')" width="120">
        <template #default="scope">
          <el-tag
            size="small"
            :type="
              String(scope.row.item_type).toUpperCase() === 'INVOICE'
                ? ''
                : String(scope.row.item_type).toUpperCase() === 'RECEIPT'
                  ? 'success'
                  : 'warning'
            "
          >
            {{ scope.row.item_type }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="document_no"
        :label="$t('arReconciliationModule.sourceNo')"
        width="150"
      />
      <el-table-column
        prop="document_date"
        :label="$t('arReconciliationModule.date')"
        width="120"
      />
      <el-table-column
        prop="amount"
        :label="$t('arReconciliationModule.amount')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.amount) }}</template>
      </el-table-column>
      <el-table-column
        prop="matched_amount"
        :label="$t('arReconciliationModule.matchedAmount')"
        width="120"
        align="right"
      >
        <template #default="scope">{{ fmtNum(scope.row.matched_amount) }}</template>
      </el-table-column>
      <el-table-column :label="$t('common.status')" width="120">
        <template #default="scope">
          <el-tag size="small" :type="getMatchType(scope.row.match_status)">
            {{ $t(getMatchLabel(scope.row.match_status)) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="remarks" :label="$t('arReconciliationModule.remark')" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type {
  AutoReconciliationResult,
  ReconciliationDetailItem,
} from '@/api/ar-reconciliation-enhanced';
import { getMatchLabel, getMatchType } from '../composables/arRecFmts';

// 金额单元格统一转数字格式化（后端 Decimal 序列化为字符串，直接 toFixed 会崩溃）
const fmtNum = (v: unknown): string => Number(v ?? 0).toFixed(2);

const { t } = useI18n({ useScope: 'global' });
void t;

/**
 * 对账明细对话框组件
 */
const props = defineProps<{
  visible: boolean;
  currentReconciliation: AutoReconciliationResult | null;
  detailData: ReconciliationDetailItem[];
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

void props;
</script>

<style scoped>
.detail-header {
  margin-bottom: 16px;
}
</style>
