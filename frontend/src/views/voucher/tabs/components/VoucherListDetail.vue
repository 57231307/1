<!--
  VoucherListDetail.vue - 凭证详情对话框
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <ElDialog
    :model-value="visible"
    :title="t('voucher.voucherListDetail.dialogTitle')"
    width="800px"
    :aria-label="t('voucher.voucherListDetail.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="viewData" class="voucher-detail">
      <div class="voucher-header">
        <div class="header-left">
          <span class="voucher-no">{{ viewData.voucher_no }}</span>
          <!-- P0 修复：真实键 voucher_type，展示后端词表 code（记/收/付/转） -->
          <span class="voucher-type">{{ viewData.voucher_type }}</span>
        </div>
        <div class="header-right">
          <span>{{ viewData.voucher_date }}</span>
          <span :class="['status-tag', getStatusClass(viewData.status)]">
            {{ getStatusLabel(viewData.status) }}
          </span>
        </div>
      </div>
      <!-- P0 修复：移除 viewData.description 展示——后端 vouchers 表无该列，历史恒不渲染 -->
      <div class="entries-table">
        <div class="entries-header">
          <span class="col-subject">{{ t('voucher.voucherListDetail.columnSubject') }}</span>
          <span class="col-debit">{{ t('voucher.voucherListDetail.columnDebit') }}</span>
          <span class="col-credit">{{ t('voucher.voucherListDetail.columnCredit') }}</span>
          <span class="col-desc">{{ t('voucher.voucherListDetail.columnSummary') }}</span>
        </div>
        <div v-for="(entry, index) in viewData.entries" :key="index" class="entries-row">
          <span class="col-subject"
            >{{ entry.account_subject_code }} - {{ entry.account_subject_name }}</span
          >
          <span class="col-debit">{{ Number(entry.debit_amount ?? 0).toFixed(2) }}</span>
          <span class="col-credit">{{ Number(entry.credit_amount ?? 0).toFixed(2) }}</span>
          <span class="col-desc">{{ entry.description || '-' }}</span>
        </div>
      </div>
      <!-- 借贷合计：后端无合计列，由详情 entries 前端派生（真实分录求和） -->
      <div class="total-row">
        <div class="total-item">
          <span class="label">{{ t('voucher.voucherListDetail.labelDebitTotal') }}</span>
          <span class="value debit">{{ totalDebit.toFixed(2) }}</span>
        </div>
        <div class="total-item">
          <span class="label">{{ t('voucher.voucherListDetail.labelCreditTotal') }}</span>
          <span class="value credit">{{ totalCredit.toFixed(2) }}</span>
        </div>
      </div>
      <ElDescriptions :column="3" border class="voucher-meta">
        <ElDescriptionsItem :label="t('voucher.voucherListDetail.labelApprovedBy')">{{
          viewData.reviewed_by ?? '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="t('voucher.voucherListDetail.labelPostedBy')">{{
          viewData.posted_by ?? '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="t('voucher.voucherListDetail.labelApprovedAt')">{{
          viewData.reviewed_at || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="t('voucher.voucherListDetail.labelPostedAt')">{{
          viewData.posted_at || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="t('voucher.voucherListDetail.labelCreatedAt')">{{
          viewData.created_at || '-'
        }}</ElDescriptionsItem>
      </ElDescriptions>
    </div>
  </ElDialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { VoucherEntity } from '@/api/voucher';
import { getStatusClass } from '../composables/vchrLstFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 凭证详情对话框组件
 * 仅做展示，对话框状态由父组件控制
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 当前凭证详情
  viewData: VoucherEntity | null;
}>();

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean];
}>();

/** 借贷合计：后端无合计列，由详情真实分录 entries 派生 */
const totalDebit = computed(() =>
  (props.viewData?.entries ?? []).reduce((sum, e) => sum + Number(e.debit_amount ?? 0), 0)
);
const totalCredit = computed(() =>
  (props.viewData?.entries ?? []).reduce((sum, e) => sum + Number(e.credit_amount ?? 0), 0)
);

/** 状态 → 国际化标签（词表对齐后端 status::finance::voucher：draft/submitted/reviewed/posted） */
const getStatusLabel = (value: string) => {
  const map: Record<string, string> = {
    draft: t('voucher.voucherListDetail.statusDraft'),
    submitted: t('voucher.voucherListDetail.statusSubmitted'),
    reviewed: t('voucher.voucherListDetail.statusReviewed'),
    posted: t('voucher.voucherListDetail.statusPosted'),
  };
  return map[value] || value;
};
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
