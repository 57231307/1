<!--
  StockTab.vue - 库存台账 Tab
  来源：原 inventory/index.vue 中 库存台账 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="stock-tab">
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="产品编码/名称"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="仓库">
          <el-select
            v-model="queryParams.warehouse_id"
            placeholder="选择仓库"
            clearable
            @change="handleQuery"
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
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="正常" value="normal" />
            <el-option label="预警" value="warning" />
            <el-option label="冻结" value="frozen" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="stocks" stripe>
        <el-table-column prop="product_code" label="产品编码" width="140" fixed />
        <el-table-column prop="product_name" label="产品名称" min-width="180" fixed />
        <el-table-column prop="warehouse_name" label="仓库" width="120" />
        <el-table-column prop="batch_no" label="批次号" width="120" />
        <el-table-column prop="color_code" label="颜色编码" width="100" />
        <el-table-column prop="location" label="库位" width="100" />
        <el-table-column prop="quantity" label="库存数量" width="100" align="right">
          <template #default="{ row }">
            <span :class="{ 'low-stock': row.quantity < (row.min_quantity ?? 0) }">
              {{ row.quantity }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="unit" label="单位" width="60" />
        <el-table-column prop="gram_weight" label="克重" width="80" />
        <el-table-column prop="width" label="门幅" width="80" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">详情</el-button>
            <el-button type="warning" link size="small" @click="handleAdjust(row)">调整</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, defineExpose } from 'vue'
import { ElMessage } from 'element-plus'
import { Search, Refresh } from '@element-plus/icons-vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

interface Warehouse {
  id: number
  warehouse_name?: string
  name?: string
}

interface StockRow {
  id: number
  product_id: number
  warehouse_id: number
  product_code: string
  product_name: string
  warehouse_name: string
  batch_no?: string
  color_code?: string
  location?: string
  quantity: number
  min_quantity?: number
  unit?: string
  gram_weight?: string
  width?: string
  status: string
}

const stocks = ref<StockRow[]>([])
const warehouses = ref<Warehouse[]>([])
const loading = ref(false)
const total = ref(0)

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  warehouse_id: undefined as number | undefined,
  status: '',
})

const getStatusType = (status: string) => {
  const typeMap: Record<string, string> = {
    normal: 'success',
    warning: 'warning',
    frozen: 'info',
  }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    normal: '正常',
    warning: '预警',
    frozen: '冻结',
  }
  return textMap[status] || status
}

const fetchData = async () => {
  loading.value = true
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getStockList(queryParams)
    const list = (res.data as unknown as StockRow[]) || []
    stocks.value = list
    total.value = list.length
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取库存列表失败')
    stocks.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchWarehouses = async () => {
  try {
    const { warehouseApi } = await import('@/api/warehouse')
    const res = await warehouseApi.list({ page: 1, page_size: 1000 })
    warehouses.value = res.data?.list || []
  } catch (error) {
    const err = error as Error
    logger.error('获取仓库列表失败', err.message)
    warehouses.value = []
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.warehouse_id = undefined
  queryParams.status = ''
  handleQuery()
}

const handleView = (row: StockRow) => {
  ElMessage.info(`查看 ${row.product_name} 详情`)
}

const handleAdjust = (_row: StockRow) => {
  ElMessage.info('请使用顶部"库存调整"按钮')
}

defineExpose({ fetchData })

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchData()
  loadIfNot('warehouses', fetchWarehouses, hasLoaded)
})
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
.low-stock {
  color: #f56c6c;
  font-weight: 600;
}
</style>
