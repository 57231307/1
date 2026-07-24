<!--
  PurchasePriceHistory.vue - 采购价格历史记录对话框
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="价格历史"
    width="800px"
    aria-label="采购价格历史对话框"
    @update:model-value="onVisibleChange"
  >
    <el-table :data="historyList" border stripe aria-label="采购价格历史记录列表">
      <el-table-column prop="price" label="采购价格" width="120" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
      </el-table-column>
      <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
      <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="180" align="center" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import type { PurchasePrice } from '@/api/purchase-price'
import { formatCurrency, getStatusType, getStatusLabel } from '../composables/ppFmts'

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
