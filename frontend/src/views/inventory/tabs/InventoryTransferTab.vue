<!--
  InventoryTransferTab.vue - 库存调拨 Tab
  来源：原 inventory/index.vue 中 transfer tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <el-card shadow="hover">
    <div class="transfer-actions">
      <el-button type="primary" @click="$emit('new-transfer')">
        <el-icon><Plus /></el-icon>
        {{ t('inventory.transferTab.newTransfer') }}
      </el-button>
    </div>
    <el-table :data="transfers" stripe :aria-label="t('inventory.transferTab.listAria')">
      <el-table-column prop="transfer_no" :label="t('inventory.transferTab.colTransferNo')" width="160" />
      <el-table-column prop="from_warehouse_name" :label="t('inventory.transferTab.colFromWarehouse')" width="120" />
      <el-table-column prop="to_warehouse_name" :label="t('inventory.transferTab.colToWarehouse')" width="120" />
      <el-table-column prop="total_quantity" :label="t('inventory.transferTab.colQuantity')" width="100" align="right" />
      <el-table-column prop="status" :label="t('inventory.transferTab.colStatus')" width="100">
        <template #default="{ row }">
          <el-tag :type="getTransferStatusType(row.status)" size="small">
            {{ getTransferStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="creator_name" :label="t('inventory.transferTab.colCreator')" width="100" />
      <el-table-column prop="created_at" :label="t('inventory.transferTab.colCreatedAt')" width="160" />
      <el-table-column :label="t('inventory.transferTab.colOperation')" width="150">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="$emit('view-transfer', row)"
            >{{ t('inventory.transferTab.detail') }}</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="$emit('approve-transfer', row)"
            >{{ t('inventory.transferTab.approve') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Plus } from '@element-plus/icons-vue'
// v11 批次 160 P2-7 修复：导入 InventoryTransfer 接口替代 any[]
import type { InventoryTransfer } from '@/api/inventory'

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' })

defineProps<{
  transfers: InventoryTransfer[]
}>()

defineEmits<{
  'new-transfer': []
  'view-transfer': [row: InventoryTransfer]
  'approve-transfer': [row: InventoryTransfer]
}>()

const getTransferStatusType = (status: string) => {
  const typeMap: Record<string, 'warning' | 'success' | 'primary' | 'info' | 'danger'> = {
    pending: 'warning',
    approved: 'success',
    executed: 'primary',
    cancelled: 'info',
  }
  return typeMap[status] || 'info'
}

// 状态标签映射函数化响应式求值
const getTransferStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    pending: t('inventory.transferTab.statusPending'),
    approved: t('inventory.transferTab.statusApproved'),
    executed: t('inventory.transferTab.statusExecuted'),
    cancelled: t('inventory.transferTab.statusCancelled'),
  }
  return textMap[status] || status
}
</script>

<style scoped>
.transfer-actions {
  margin-bottom: 16px;
}
</style>
