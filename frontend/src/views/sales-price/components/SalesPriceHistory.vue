<!--
  SalesPriceHistory.vue - 销售价格历史记录对话框
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('salesPrice.history.dialogTitle')"
    width="800px"
    :aria-label="t('salesPrice.history.dialogAriaLabel')"
    @update:model-value="onVisibleChange"
  >
    <el-table
      :data="historyList"
      border
      stripe
      :aria-label="t('salesPrice.history.tableAriaLabel')"
    >
      <el-table-column
        prop="price"
        :label="t('salesPrice.history.columnPrice')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="effective_date"
        :label="t('salesPrice.history.columnEffectiveDate')"
        width="120"
        align="center"
      />
      <el-table-column
        prop="expiry_date"
        :label="t('salesPrice.history.columnExpiryDate')"
        width="120"
        align="center"
      />
      <el-table-column
        prop="status"
        :label="t('salesPrice.history.columnStatus')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="created_at"
        :label="t('salesPrice.history.columnCreatedAt')"
        width="180"
        align="center"
      />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SalesPrice } from '@/api/sales-price';
// 状态文案统一走 spFmts（词表 models/status/sales.rs::price_approval，i18n 键+未知 token 抛错），删除组件局部 map
import { formatCurrency, getStatusType, getStatusLabel } from '../composables/spFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 销售价格历史记录对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean;
  // 历史记录列表
  historyList: SalesPrice[];
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>
