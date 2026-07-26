<!--
  PurchasePriceHistory.vue - 采购价格历史记录对话框
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchasePrice.history.title')"
    width="800px"
    :aria-label="t('purchasePrice.history.ariaLabel.dialog')"
    @update:model-value="onVisibleChange"
  >
    <el-table :data="historyList" border stripe :aria-label="t('purchasePrice.history.ariaLabel.table')">
      <el-table-column prop="price" :label="t('purchasePrice.history.column.price')" width="120" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
      </el-table-column>
      <el-table-column prop="effective_date" :label="t('purchasePrice.history.column.effectiveDate')" width="120" align="center" />
      <el-table-column prop="expiry_date" :label="t('purchasePrice.history.column.expiryDate')" width="120" align="center" />
      <el-table-column prop="status" :label="t('purchasePrice.history.column.status')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" :label="t('purchasePrice.history.column.createdAt')" width="180" align="center" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { PurchasePrice } from '@/api/purchase-price'
import { formatCurrency, getStatusType, getStatusLabel } from '../composables/ppFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 采购价格历史记录对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 历史记录列表
  historyList: PurchasePrice[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>
