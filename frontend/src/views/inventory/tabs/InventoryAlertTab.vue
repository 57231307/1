<!--
  InventoryAlertTab.vue - 库存预警 Tab
  来源：原 inventory/index.vue 中 alert tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <el-card shadow="hover">
    <el-table :data="alerts" stripe :aria-label="t('inventory.alertTab.listAria')">
      <el-table-column
        prop="product_code"
        :label="t('inventory.alertTab.colProductCode')"
        width="140"
      />
      <el-table-column
        prop="product_name"
        :label="t('inventory.alertTab.colProductName')"
        min-width="180"
      />
      <el-table-column
        prop="warehouse_name"
        :label="t('inventory.alertTab.colWarehouse')"
        width="120"
      />
      <el-table-column
        prop="quantity_on_hand"
        :label="t('inventory.alertTab.colCurrentQty')"
        width="100"
        align="right"
      >
        <template #default="{ row }">
          <span class="low-stock">{{ formatNumber(Number(row.quantity_on_hand)) }}</span>
        </template>
      </el-table-column>
      <el-table-column
        prop="reorder_point"
        :label="t('inventory.alertTab.colMinQty')"
        width="100"
        align="right"
      />
      <el-table-column prop="unit" :label="t('inventory.alertTab.colUnit')" width="60" />
      <!-- 告警类型是后端派生码（normal/low_stock/...），不再是自造的 warning|danger 两档 -->
      <el-table-column prop="alert_type" :label="t('inventory.alertTab.colAlertLevel')" width="150">
        <template #default="{ row }">
          <el-tag :type="alertTypeTagType(row.alert_type)" size="small">
            {{ alertTypeText(row.alert_type) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('inventory.alertTab.colOperation')" width="100">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="$emit('purchase', row)">{{
            t('inventory.alertTab.purchase')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
// v11 批次 160 P2-7 修复：导入 StockAlert 接口替代 any[]
import type { StockAlert } from '@/api/inventory';
import {
  STOCK_ALERT_TYPE_LABEL_KEY,
  STOCK_ALERT_TYPE_TAG_TYPES,
  type StockAlertTypeValue,
} from '@/constants/stock-alert-type';
import { formatNumber } from '../composables/invFmts';
import { logger } from '@/utils/logger';

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' });

/** 告警类型 → 文案：后端派生码（AlertType）在取值域外时告警并原样显示，不猜含义 */
const alertTypeText = (alertType: string) => {
  const key = STOCK_ALERT_TYPE_LABEL_KEY[alertType as StockAlertTypeValue];
  if (!key) {
    logger.warn(`未知库存告警类型，需与后端 AlertType 取值域同步：${alertType}`);
    return alertType;
  }
  return t(key);
};

const alertTypeTagType = (alertType: string) =>
  STOCK_ALERT_TYPE_TAG_TYPES[alertType as StockAlertTypeValue] ?? 'warning';

defineProps<{
  alerts: StockAlert[];
}>();

defineEmits<{
  purchase: [row: StockAlert];
}>();
</script>

<style scoped>
.low-stock {
  color: #f56c6c;
  font-weight: 600;
}
</style>
