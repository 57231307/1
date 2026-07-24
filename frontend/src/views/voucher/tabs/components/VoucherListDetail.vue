<!--
  VoucherListDetail.vue - 凭证详情对话框
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <ElDialog
    :model-value="visible"
    title="凭证详情"
    width="800px"
    aria-label="凭证详情对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="viewData" class="voucher-detail">
      <div class="voucher-header">
        <div class="header-left">
          <span class="voucher-no">{{ viewData.voucher_no }}</span>
          <span class="voucher-type">{{ getTypeLabel(viewData.type) }}</span>
        </div>
        <div class="header-right">
          <span>{{ viewData.voucher_date }}</span>
          <span :class="['status-tag', getStatusClass(viewData.status)]">
            {{ getStatusLabel(viewData.status) }}
          </span>
        </div>
      </div>
      <div v-if="viewData.description" class="voucher-desc">{{ viewData.description }}</div>
      <div class="entries-table">
        <div class="entries-header">
          <span class="col-subject">会计科目</span>
          <span class="col-debit">借方金额</span>
          <span class="col-credit">贷方金额</span>
          <span class="col-desc">摘要</span>
        </div>
        <div v-for="(entry, index) in viewData.entries" :key="index" class="entries-row">
          <span class="col-subject"
            >{{ entry.account_subject_code }} - {{ entry.account_subject_name }}</span
          >
          <span class="col-debit">{{ entry.debit_amount.toFixed(2) }}</span>
          <span class="col-credit">{{ entry.credit_amount.toFixed(2) }}</span>
          <span class="col-desc">{{ entry.description || '-' }}</span>
        </div>
      </div>
      <div class="total-row">
        <div class="total-item">
          <span class="label">借方合计:</span>
          <span class="value debit">{{ viewData.total_debit.toFixed(2) }}</span>
        </div>
        <div class="total-item">
          <span class="label">贷方合计:</span>
          <span class="value credit">{{ viewData.total_credit.toFixed(2) }}</span>
        </div>
      </div>
      <ElDescriptions :column="3" border class="voucher-meta">
        <ElDescriptionsItem label="制单人">{{
          viewData.created_by_name || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem label="审核人">{{
          viewData.approved_by_name || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem label="记账人">{{
          viewData.posted_by_name || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem label="审核时间">{{
          viewData.approved_at || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem label="记账时间">{{ viewData.posted_at || '-' }}</ElDescriptionsItem>
        <ElDescriptionsItem label="创建时间">{{ viewData.created_at || '-' }}</ElDescriptionsItem>
      </ElDescriptions>
    </div>
  </ElDialog>
</template>

<script setup lang="ts">
import type { VoucherEntity } from '@/api/voucher'
import { getStatusLabel, getStatusClass, getTypeLabel } from '../composables/vchrLstFmts'

/**
 * 凭证详情对话框组件
 * 仅做展示，对话框状态由父组件控制
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 当前凭证详情
  viewData: VoucherEntity | null
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
}>()

void props
</script>

<style scoped>
.voucher-detail {
  padding: 20px;
}
.voucher-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}
.voucher-no {
  font-size: 20px;
  font-weight: bold;
}
.voucher-type {
  margin-left: 10px;
  color: #666;
}
.voucher-desc {
  padding: 10px;
  background: #f5f7fa;
  margin-bottom: 10px;
}
.voucher-meta {
  margin-top: 20px;
}
.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}
.status-draft {
  background: #f5f7fa;
  color: #909399;
}
.status-approved {
  background: #e6f7ff;
  color: #1890ff;
}
.status-posted {
  background: #f0f9eb;
  color: #67c23a;
}
.entries-table {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}
.entries-header {
  display: flex;
  background: #f5f7fa;
  padding: 10px;
  font-weight: bold;
}
.entries-row {
  display: flex;
  padding: 10px;
  border-top: 1px solid #ebeef5;
}
.col-subject {
  flex: 2;
  margin-right: 10px;
}
.col-debit,
.col-credit {
  width: 120px;
  margin-right: 10px;
}
.col-desc {
  flex: 1;
  margin-right: 10px;
}
.total-row {
  display: flex;
  justify-content: flex-end;
  padding: 10px;
  background: #fafafa;
  margin-top: 10px;
}
.total-item {
  margin-left: 30px;
}
.total-item .label {
  margin-right: 10px;
  font-weight: bold;
}
.total-item .value {
  font-weight: bold;
  font-size: 16px;
}
.total-item .value.debit {
  color: #e74c3c;
}
.total-item .value.credit {
  color: #27ae60;
}
</style>
