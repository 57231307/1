<!--
  InventoryStockTab.vue - 库存台账 Tab
  来源：原 inventory/index.vue 中 stock tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <div>
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="localQuery"
        class="filter-form"
        :aria-label="t('inventory.stockTab.filterAria')"
      >
        <el-form-item :label="t('inventory.stockTab.keyword')">
          <el-input
            v-model="localQuery.keyword"
            :placeholder="t('inventory.stockTab.keywordPlaceholder')"
            clearable
            @clear="emit('query')"
          />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.warehouse')">
          <el-select
            v-model="localQuery.warehouse_id"
            :placeholder="t('inventory.stockTab.warehousePlaceholder')"
            clearable
            @change="emit('query')"
          >
            <el-option
              v-for="wh in warehouses"
              :key="wh.id"
              :label="wh.warehouse_name"
              :value="wh.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.status')">
          <el-select
            v-model="localQuery.stock_status"
            :placeholder="t('inventory.stockTab.statusPlaceholder')"
            clearable
            @change="emit('query')"
          >
            <el-option
              v-for="opt in INVENTORY_STOCK_STATUS_OPTIONS"
              :key="opt"
              :label="t(INVENTORY_STOCK_STATUS_LABEL_KEY[opt])"
              :value="opt"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="emit('query')">
            <el-icon><Search /></el-icon>
            {{ t('inventory.stockTab.query') }}
          </el-button>
          <el-button @click="emit('reset')">
            <el-icon><Refresh /></el-icon>
            {{ t('inventory.stockTab.reset') }}
          </el-button>
          <el-button type="success" @click="emit('create')">
            {{ t('inventory.stockTab.create') }}
          </el-button>
          <el-button @click="emit('export')">{{ t('inventory.stockTab.export') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <div class="table-toolbar">
        <el-button size="small" @click="emit('create')">
          {{ t('inventory.stockTab.create') }}
        </el-button>
      </div>
      <V2Table
        :data="stocks"
        :columns="stockColumns"
        :estimated-row-height="40"
        :loading="loading"
        :total="total"
        :page="localQuery.page"
        :page-size="localQuery.page_size"
        @row-click="(row: InventoryStock) => emit('view', row)"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { h, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElButton } from 'element-plus';
import { Search, Refresh } from '@element-plus/icons-vue';
import V2Table from '@/components/V2Table/index.vue';
import { useTableColumns } from '@/composables/useTableColumns';
import { getStockStatusLabel } from '../composables/invFmts';
import {
  INVENTORY_STOCK_STATUS_LABEL_KEY,
  INVENTORY_STOCK_STATUS_OPTIONS,
} from '@/constants/inventory-stock-status';
// v11 批次 160 P2-7 修复：导入具体接口类型替代 any[]
import type { InventoryStock } from '@/api/inventory';
import type { Warehouse } from '@/api/warehouse';

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' });

export interface StockQuery {
  page: number;
  page_size: number;
  keyword: string;
  warehouse_id: number | undefined;
  stock_status: string;
}

const props = defineProps<{
  stocks: InventoryStock[];
  total: number;
  loading: boolean;
  queryParams: StockQuery;
  warehouses: Warehouse[];
}>();

const emit = defineEmits<{
  view: [row: InventoryStock];
  query: [];
  reset: [];
  create: [];
  edit: [row: InventoryStock];
  delete: [row: InventoryStock];
  export: [];
  'update:queryParams': [value: StockQuery];
}>();

const localQuery = reactive<StockQuery>({ ...props.queryParams });

watch(
  () => props.queryParams,
  newParams => {
    Object.assign(localQuery, newParams);
  },
  { deep: true }
);

// 状态标签映射：与详情页、打印共用同一份取值→文案映射（见 composables/invFmts）
const getStatusText = (status: string) => getStockStatusLabel(status, t);

const { columns: stockColumns } = useTableColumns<InventoryStock>([
  {
    key: 'product_code',
    title: t('inventory.stockTab.colProductCode'),
    width: 140,
    sortable: true,
  },
  { key: 'product_name', title: t('inventory.stockTab.colProductName'), width: 200 },
  { key: 'warehouse_name', title: t('inventory.stockTab.colWarehouse'), width: 120 },
  { key: 'batch_no', title: t('inventory.stockTab.colBatchNo'), width: 120 },
  { key: 'color_no', title: t('inventory.stockTab.colColorCode'), width: 100 },
  { key: 'dye_lot_no', title: t('inventory.stockTab.colDyeLot'), width: 110 },
  {
    key: 'quantity_on_hand',
    title: t('inventory.stockTab.colQuantity'),
    width: 120,
    align: 'right',
    formatter: (row: InventoryStock) =>
      row.quantity_on_hand != null ? Number(row.quantity_on_hand).toLocaleString() : '-',
  },
  {
    key: 'stock_status',
    title: t('inventory.stockTab.colStatus'),
    width: 100,
    align: 'center',
    formatter: (row: InventoryStock) => getStatusText(row.stock_status),
  },
  { key: 'bin_location', title: t('inventory.stockTab.colLocation'), width: 100 },
  {
    key: 'operation',
    title: t('inventory.stockTab.colOperation'),
    width: 120,
    fixed: 'right',
    // V2Table 通过 renderCell 渲染操作列（无插槽机制）
    renderCell: (row: InventoryStock) =>
      h('div', { class: 'operation-cell' }, [
        h(
          ElButton,
          {
            size: 'small',
            type: 'primary',
            link: true,
            onClick: (e: Event) => {
              e.stopPropagation();
              emit('edit', row);
            },
          },
          () => t('common.edit')
        ),
        h(
          ElButton,
          {
            size: 'small',
            type: 'danger',
            link: true,
            onClick: (e: Event) => {
              e.stopPropagation();
              emit('delete', row);
            },
          },
          () => t('common.delete')
        ),
      ]),
  },
]);

const handlePageChange = (newPage: number) => {
  emit('update:queryParams', { ...localQuery, page: newPage });
  emit('query');
};

const handleSizeChange = (newSize: number) => {
  emit('update:queryParams', { ...localQuery, page_size: newSize, page: 1 });
  emit('query');
};
</script>

<style scoped>
.filter-card,
.table-card {
  margin-bottom: 16px;
}
</style>
