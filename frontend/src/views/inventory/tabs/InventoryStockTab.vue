<!--
  InventoryStockTab.vue - 库存台账 Tab
  来源：原 inventory/index.vue 中 stock tab 区
  拆分日期：2026-06-17 P1-3-Batch-3
-->
<template>
  <div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="localQuery" class="filter-form" aria-label="库存台账筛选表单">
        <el-form-item label="关键词">
          <el-input
            v-model="localQuery.keyword"
            placeholder="产品编码/名称"
            clearable
            @clear="emit('query')"
          />
        </el-form-item>
        <el-form-item label="仓库">
          <el-select
            v-model="localQuery.warehouse_id"
            placeholder="选择仓库"
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
        <el-form-item label="状态">
          <el-select
            v-model="localQuery.status"
            placeholder="选择状态"
            clearable
            @change="emit('query')"
          >
            <el-option label="正常" value="normal" />
            <el-option label="预警" value="warning" />
            <el-option label="冻结" value="frozen" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="emit('query')">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="emit('reset')">
            <el-icon><Refresh /></el-icon>
            重置
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
import { Search, Refresh } from '@element-plus/icons-vue'
import V2Table from '@/components/V2Table/index.vue'
import { useTableColumns } from '@/composables/useTableColumns'
// v11 批次 160 P2-7 修复：导入具体接口类型替代 any[]
import type { InventoryStock } from '@/api/inventory'
import type { Warehouse } from '@/api/warehouse'

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

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    normal: '正常',
    warning: '预警',
    frozen: '冻结',
  }
  return textMap[status] || status
}

const { columns: stockColumns } = useTableColumns<InventoryStock>([
  { key: 'product_code', title: '产品编码', width: 140, sortable: true },
  { key: 'product_name', title: '产品名称', width: 200 },
  { key: 'warehouse_name', title: '仓库', width: 120 },
  { key: 'batch_no', title: '批次号', width: 120 },
  { key: 'color_code', title: '颜色编码', width: 100 },
  {
    key: 'quantity',
    title: '库存数量',
    width: 120,
    align: 'right',
    formatter: (row: InventoryStock) => (row.quantity != null ? row.quantity.toLocaleString() : '-'),
  },
  {
    key: 'status',
    title: '状态',
    width: 100,
    align: 'center',
    formatter: (row: InventoryStock) => getStatusText(row.status),
  },
  { key: 'location', title: '库位', width: 100 },
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
