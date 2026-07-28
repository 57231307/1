<template>
  <!--
    色卡发放详情展示组件（V15 P0-F12）
    展示单条发放记录的全字段，供详情对话框/抽屉使用
  -->
  <el-descriptions :column="2" border :aria-label="t('components.colorCardIssueDetail.ariaLabel')">
    <el-descriptions-item :label="t('components.colorCardIssueDetail.recordId')">{{
      record.id
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.colorCardId')">{{
      record.color_card_id
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.customerId')">{{
      record.customer_id
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.issueQty')">{{
      record.issue_qty
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.issuedBy')">{{
      record.issued_by
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.issuedAt')">{{
      formatDate(record.issued_at)
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.expectedReturn')">{{
      formatDate(record.expected_return_date)
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.actualReturn')">{{
      formatDate(record.actual_return_date)
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.status')">
      <el-tag :type="statusColor">{{ statusLabel }}</el-tag>
    </el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.dyeLotNo')">{{
      record.dye_lot_no || '-'
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.purpose')" :span="2">{{
      record.purpose || '-'
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.remark')" :span="2">{{
      record.remark || '-'
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.compensationAmount')">
      {{ record.compensation_amount != null ? `¥${record.compensation_amount.toFixed(2)}` : '-' }}
    </el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.returnedBy')">{{
      record.returned_by ?? '-'
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.createdAt')">{{
      formatDate(record.created_at)
    }}</el-descriptions-item>
    <el-descriptions-item :label="t('components.colorCardIssueDetail.updatedAt')">{{
      formatDate(record.updated_at)
    }}</el-descriptions-item>
  </el-descriptions>
</template>

<script setup lang="ts">
// 色卡发放详情展示组件（V15 P0-F12）
// 创建时间：2026-07-18（Batch 477 P0-F12）

import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { IssueRecordInfo, IssueStatusValue } from '@/types/colorCardIssue';

const props = defineProps<{
  record: IssueRecordInfo;
}>();

const { t } = useI18n({ useScope: 'global' });

// 发放状态标签映射 - 改为响应式求值（依赖 t() 切换语言时自动更新）
const STATUS_LABEL_KEYS: Record<IssueStatusValue, string> = {
  issued: 'components.colorCardIssueDetail.statusLabels.issued',
  returned: 'components.colorCardIssueDetail.statusLabels.returned',
  lost: 'components.colorCardIssueDetail.statusLabels.lost',
  damaged: 'components.colorCardIssueDetail.statusLabels.damaged',
  cancelled: 'components.colorCardIssueDetail.statusLabels.cancelled',
};

// 发放状态颜色映射（Element Plus Tag type）
const STATUS_COLORS: Record<IssueStatusValue, 'warning' | 'success' | 'danger' | 'info'> = {
  issued: 'warning',
  returned: 'success',
  lost: 'danger',
  damaged: 'danger',
  cancelled: 'info',
};

const statusLabel = computed(() => {
  const key = STATUS_LABEL_KEYS[props.record.status];
  return key ? t(key) : props.record.status;
});
const statusColor = computed(() => STATUS_COLORS[props.record.status] || 'info');

const formatDate = (s?: string): string => {
  if (!s) return '-';
  try {
    return new Date(s).toLocaleString('zh-CN');
  } catch {
    return s;
  }
};
</script>
