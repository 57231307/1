<!--
  InventoryAlertTab.vue - 库存预警 Tab
  来源：原 inventory/index.vue 中 alert tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <el-card shadow="hover">
    <el-table :data="alerts" stripe :aria-label="t('inventory.alertTab.listAria')">
      <el-table-column prop="product_code" :label="t('inventory.alertTab.colProductCode')" width="140" />
      <el-table-column prop="product_name" :label="t('inventory.alertTab.colProductName')" min-width="180" />
      <el-table-column prop="warehouse_name" :label="t('inventory.alertTab.colWarehouse')" width="120" />
      <el-table-column prop="current_quantity" :label="t('inventory.alertTab.colCurrentQty')" width="100" align="right">
        <template #default="{ row }">
          <span class="low-stock">{{ row.current_quantity }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="min_quantity" :label="t('inventory.alertTab.colMinQty')" width="100" align="right" />
      <el-table-column prop="unit" :label="t('inventory.alertTab.colUnit')" width="60" />
      <el-table-column prop="alert_level" :label="t('inventory.alertTab.colAlertLevel')" width="100">
        <template #default="{ row }">
          <el-tag :type="row.alert_level === 'danger' ? 'danger' : 'warning'" size="small">
            {{ row.alert_level === 'danger' ? t('inventory.alertTab.urgent') : t('inventory.alertTab.warning') }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('inventory.alertTab.colOperation')" width="100">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="$emit('purchase', row)"
            >{{ t('inventory.alertTab.purchase') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
// v11 批次 160 P2-7 修复：导入 StockAlert 接口替代 any[]
import type { StockAlert } from '@/api/inventory'

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' })

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
