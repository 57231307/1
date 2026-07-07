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
        新建调拨单
      </el-button>
    </div>
    <el-table :data="transfers" stripe>
      <el-table-column prop="transfer_no" label="调拨单号" width="160" />
      <el-table-column prop="from_warehouse_name" label="调出仓库" width="120" />
      <el-table-column prop="to_warehouse_name" label="调入仓库" width="120" />
      <el-table-column prop="total_quantity" label="调拨数量" width="100" align="right" />
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getTransferStatusType(row.status)" size="small">
            {{ getTransferStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="creator_name" label="创建人" width="100" />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="150">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="$emit('view-transfer', row)"
            >详情</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="$emit('approve-transfer', row)"
            >审批</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'
// v11 批次 160 P2-7 修复：导入 InventoryTransfer 接口替代 any[]
import type { InventoryTransfer } from '@/api/inventory'

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

const getTransferStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    executed: '已执行',
    cancelled: '已取消',
  }
  return textMap[status] || status
}
</script>

<style scoped>
.transfer-actions {
  margin-bottom: 16px;
}
</style>
