<!--
  StockTab.vue - 库存台账 Tab（V2Table 迁移版）
  任务编号: Wave 4 P2-1 PR-2
  关联 spec: docs/superpowers/specs/2026-06-16-wave4-p2-1-design.md
  拆分日期：2026-06-15 B3-4
  迁移日期：2026-06-16 P2-1
-->
<template>
  <div class="stock-tab">
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="localQueryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="localQueryParams.keyword"
            placeholder="产品编码/名称"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="仓库">
          <el-select
            v-model="localQueryParams.warehouse_id"
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
            v-model="localQueryParams.status"
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
      <V2Table
        :columns="columns"
        :data="data"
        :loading="loading"
        :page="page"
        :page-size="pageSize"
        :total="total"
        :height="600"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
        @row-click="handleRowClick"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
/**
 * 库存台账 Tab（V2Table 迁移版）
 * - V2Table：基于 el-table-v2 的虚拟滚动通用组件
 * - useTableApi：通用数据 composable（分页/筛选/loading/重试）
 * 保留原交互：低库存红色 / 状态 el-tag / 详情+调整按钮 / 仓库下拉懒加载 / defineExpose
 */
import { ref, reactive, h, onMounted } from 'vue'
import { ElMessage, ElTag, ElButton } from 'element-plus'
import { Search, Refresh } from '@element-plus/icons-vue'
import { useTableApi } from '@/composables/useTableApi'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
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

const warehouses = ref<Warehouse[]>([])

const {
  data,
  loading,
  page,
  pageSize,
  total,
  queryParams,
  refresh,
  reset,
} = useTableApi<StockRow>('/inventory/stock')

const localQueryParams = reactive({
  keyword: '',
  warehouse_id: undefined as number | undefined,
  status: '' as string,
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

/**
 * 列定义：使用 renderCell 自定义渲染
 * - 低库存红：quantity < min_quantity 时高亮
 * - 状态 el-tag：normal/warning/frozen 配色
 * - 操作列：详情 / 调整 按钮
 */
const columns: ColumnDef[] = [
  { key: 'product_code', title: '产品编码', width: 140, fixed: 'left' },
  { key: 'product_name', title: '产品名称', minWidth: 180 },
  { key: 'warehouse_name', title: '仓库', width: 120 },
  { key: 'batch_no', title: '批次号', width: 120 },
  { key: 'color_code', title: '颜色编码', width: 100 },
  { key: 'location', title: '库位', width: 100 },
  {
    key: 'quantity',
    title: '库存数量',
    width: 100,
    align: 'right',
    renderCell: (row: StockRow) =>
      h(
        'span',
        { class: { 'low-stock': row.quantity < (row.min_quantity ?? 0) } },
        String(row.quantity)
      ),
  },
  { key: 'unit', title: '单位', width: 60 },
  { key: 'gram_weight', title: '克重', width: 80 },
  { key: 'width', title: '门幅', width: 80 },
  {
    key: 'status',
    title: '状态',
    width: 80,
    renderCell: (row: StockRow) =>
      h(
        ElTag,
        { type: getStatusType(row.status), size: 'small' },
        () => getStatusText(row.status)
      ),
  },
  {
    key: '__actions__',
    title: '操作',
    width: 150,
    fixed: 'right',
    renderCell: (row: StockRow) =>
      h('div', { class: 'action-cell' }, [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => handleView(row) },
          () => '详情'
        ),
        h(
          ElButton,
          { type: 'warning', link: true, size: 'small', onClick: () => handleAdjust(row) },
          () => '调整'
        ),
      ]),
  },
]

const fetchWarehouses = async () => {
  try {
    const { warehouseApi } = await import('@/api/warehouse')
    const res = await warehouseApi.list({ page: 1, page_size: 1000 })
    warehouses.value = (res.data?.list as Warehouse[]) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取仓库列表失败', err.message)
    warehouses.value = []
  }
}

const handleQuery = () => {
  queryParams.value = { ...queryParams.value, ...localQueryParams, page: 1 }
  page.value = 1
  refresh()
}

const handleReset = () => {
  localQueryParams.keyword = ''
  localQueryParams.warehouse_id = undefined
  localQueryParams.status = ''
  reset()
  refresh()
}

const handleView = (row: StockRow) => {
  ElMessage.info(`查看 ${row.product_name} 详情`)
}

const handleAdjust = (_row: StockRow) => {
  ElMessage.info('请使用顶部"库存调整"按钮')
}

const handlePageChange = (newPage: number) => {
  page.value = newPage
}

const handleSizeChange = (newSize: number) => {
  pageSize.value = newSize
}

const handleRowClick = (row: StockRow) => {
  handleView(row)
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  refresh()
  loadIfNot('warehouses', fetchWarehouses, hasLoaded)
})

/**
 * 暴露给父组件：刷新表格数据
 * 保留兼容：原 el-table 实现的 fetchData 入口
 */
defineExpose({ fetchData: refresh })
</script>

<style scoped>
.stock-tab {
  padding: 16px;
}
.filter-card {
  margin-bottom: 16px;
}
.table-card {
  margin-bottom: 16px;
}
.low-stock {
  color: #f56c6c;
  font-weight: 600;
}
.action-cell {
  display: flex;
  gap: 4px;
}
</style>
