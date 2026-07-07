<!--
  InventoryAlertTab.vue - 库存预警 Tab
  来源：原 inventory/index.vue 中 alert tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <el-card shadow="hover">
    <el-table :data="alerts" stripe>
      <el-table-column prop="product_code" label="产品编码" width="140" />
      <el-table-column prop="product_name" label="产品名称" min-width="180" />
      <el-table-column prop="warehouse_name" label="仓库" width="120" />
      <el-table-column prop="current_quantity" label="当前库存" width="100" align="right">
        <template #default="{ row }">
          <span class="low-stock">{{ row.current_quantity }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="min_quantity" label="最小库存" width="100" align="right" />
      <el-table-column prop="unit" label="单位" width="60" />
      <el-table-column prop="alert_level" label="预警级别" width="100">
        <template #default="{ row }">
          <el-tag :type="row.alert_level === 'danger' ? 'danger' : 'warning'" size="small">
            {{ row.alert_level === 'danger' ? '紧急' : '警告' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="100">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="$emit('purchase', row)"
            >采购</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
// v11 批次 160 P2-7 修复：导入 StockAlert 接口替代 any[]
import type { StockAlert } from '@/api/inventory'

defineProps<{
  alerts: StockAlert[]
}>()

defineEmits<{
  purchase: [row: StockAlert]
}>()
</script>

<style scoped>
.low-stock {
  color: #f56c6c;
  font-weight: 600;
}
</style>
