<!--
  InventoryStockTab.vue - 库存台账 Tab
  来源：原 inventory/index.vue 中 stock tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="localQuery" class="filter-form" :aria-label="t('inventory.stockTab.filterAria')">
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
            v-model="localQuery.status"
            :placeholder="t('inventory.stockTab.statusPlaceholder')"
            clearable
            @change="emit('query')"
          >
            <el-option :label="t('inventory.stockTab.statusNormal')" value="normal" />
            <el-option :label="t('inventory.stockTab.statusWarning')" value="warning" />
            <el-option :label="t('inventory.stockTab.statusFrozen')" value="frozen" />
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
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
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
import { reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search, Refresh } from '@element-plus/icons-vue'
import V2Table from '@/components/V2Table/index.vue'
import { useTableColumns } from '@/composables/useTableColumns'
// v11 批次 160 P2-7 修复：导入具体接口类型替代 any[]
import type { InventoryStock } from '@/api/inventory'
import type { Warehouse } from '@/api/warehouse'

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' })

export interface StockQuery {
  page: number
  page_size: number
  keyword: string
  warehouse_id: number | undefined
  status: string
}

const props = defineProps<{
  stocks: InventoryStock[]
  total: number
  loading: boolean
  queryParams: StockQuery
  warehouses: Warehouse[]
}>()

const emit = defineEmits<{
  view: [row: InventoryStock]
  query: []
  reset: []
  'update:queryParams': [value: StockQuery]
}>()

const localQuery = reactive<StockQuery>({ ...props.queryParams })

watch(
  () => props.queryParams,
  newParams => {
    Object.assign(localQuery, newParams)
  },
  { deep: true }
)

// 状态标签映射函数化响应式求值
const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    normal: t('inventory.stockTab.statusNormal'),
    warning: t('inventory.stockTab.statusWarning'),
    frozen: t('inventory.stockTab.statusFrozen'),
  }
  return textMap[status] || status
}

const { columns: stockColumns } = useTableColumns<InventoryStock>([
  { key: 'product_code', title: t('inventory.stockTab.colProductCode'), width: 140, sortable: true },
  { key: 'product_name', title: t('inventory.stockTab.colProductName'), width: 200 },
  { key: 'warehouse_name', title: t('inventory.stockTab.colWarehouse'), width: 120 },
  { key: 'batch_no', title: t('inventory.stockTab.colBatchNo'), width: 120 },
  { key: 'color_code', title: t('inventory.stockTab.colColorCode'), width: 100 },
  {
    key: 'quantity',
    title: t('inventory.stockTab.colQuantity'),
    width: 120,
    align: 'right',
    formatter: (row: InventoryStock) => (row.quantity != null ? row.quantity.toLocaleString() : '-'),
  },
  {
    key: 'status',
    title: t('inventory.stockTab.colStatus'),
    width: 100,
    align: 'center',
    formatter: (row: InventoryStock) => getStatusText(row.status),
  },
  { key: 'location', title: t('inventory.stockTab.colLocation'), width: 100 },
])

const handlePageChange = (newPage: number) => {
  emit('update:queryParams', { ...localQuery, page: newPage })
  emit('query')
}

const handleSizeChange = (newSize: number) => {
  emit('update:queryParams', { ...localQuery, page_size: newSize, page: 1 })
  emit('query')
}
</script>

<style scoped>
.filter-card,
.table-card {
  margin-bottom: 16px;
}
</style>
