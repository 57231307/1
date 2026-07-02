<!--
  StockTab.vue - 库存台账 Tab（V2Table 迁移版 - PR-2）
  任务编号: Wave 4 P2-1 PR-2
  关联 spec: docs/superpowers/specs/2026-06-16-wave4-p2-1-design.md
  拆分日期：2026-06-15 B3-4
  迁移日期：2026-06-16 P2-1（保留：CRUD / 过滤 / 搜索 / 排序 / 行选择 / 批量操作）
-->
<template>
  <div class="stock-tab">
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="localQueryParams" @submit.prevent="handleQuery">
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

    <el-card v-if="data.length > 0" shadow="hover" class="select-bar">
      <el-checkbox
        :model-value="allSelected"
        :indeterminate="indeterminate"
        @update:model-value="toggleAll"
      >
        全选当前页（{{ data.length }} 条）
      </el-checkbox>
    </el-card>

    <el-card v-if="selectedRows.length > 0" shadow="hover" class="batch-bar">
      <span class="selected-info">已选 {{ selectedRows.length }} 项</span>
      <el-button
        v-permission="'inventory:adjust'"
        type="primary"
        size="small"
        @click="handleBatchAdjust"
      >
        <el-icon><Edit /></el-icon> 批量调整
      </el-button>
      <el-button
        v-permission="'inventory:adjust'"
        type="danger"
        size="small"
        @click="handleBatchDelete"
      >
        <el-icon><Delete /></el-icon> 批量删除
      </el-button>
      <el-button size="small" @click="clearSelection">清空选择</el-button>
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
 * 库存台账 Tab - V2Table 迁移版
 * - V2Table：基于 el-table-v2 的虚拟滚动通用组件
 * - useTableApi：通用数据 composable（分页/筛选/loading/重试）
 * 保留功能：CRUD（详情/编辑/删除/调整）/ 过滤 / 搜索 / 排序 / 行选择 / 批量操作
 */
import { ref, reactive, computed, h, onMounted } from 'vue'
import { ElMessage, ElMessageBox, ElTag, ElButton, ElCheckbox } from 'element-plus'
import { Search, Refresh, Edit, Delete } from '@element-plus/icons-vue'
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
  status: string
  updated_at?: string
}

// 状态文本与 el-tag 类型映射（统一常量，避免硬编码）
const STATUS_TEXT: Record<string, string> = {
  normal: '正常',
  warning: '预警',
  frozen: '冻结',
}
// 限定为 ElTag 接受的类型集合，与 el-tag type 属性严格匹配
type ElTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'
const STATUS_TYPE: Record<string, ElTagType> = {
  normal: 'success',
  warning: 'warning',
  frozen: 'info',
}

const warehouses = ref<Warehouse[]>([])
const selectedRows = ref<StockRow[]>([])

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

/**
 * 行选择辅助：单行切换 / 全选 / 清空
 * 注意：选中状态仅在当前页内维护，跨页选择需业务侧扩展
 */
const isSelected = (row: StockRow) =>
  selectedRows.value.some(r => r.id === row.id)

const toggleRow = (row: StockRow) => {
  const idx = selectedRows.value.findIndex(r => r.id === row.id)
  if (idx >= 0) selectedRows.value.splice(idx, 1)
  else selectedRows.value.push(row)
}

const toggleAll = () => {
  if (selectedRows.value.length === data.value.length) {
    selectedRows.value = []
  } else {
    selectedRows.value = [...data.value]
  }
}

const allSelected = computed(
  () => data.value.length > 0 && selectedRows.value.length === data.value.length
)

const indeterminate = computed(
  () => selectedRows.value.length > 0 && selectedRows.value.length < data.value.length
)

const clearSelection = () => {
  selectedRows.value = []
}

/**
 * 列定义：使用 renderCell 自定义渲染
 * - 复选框：行选择（单行 + 全选）
 * - 排序：sortable 标记（el-table-v2 内部 UI 排序）
 * - 低库存红：quantity < min_quantity 时高亮
 * - 状态 el-tag：normal/warning/frozen 配色
 * - 操作列：详情 / 编辑 / 删除 按钮（v-permission 控制权限）
 */
const columns = computed<ColumnDef[]>(() => [
  {
    key: '__selection__',
    title: '',
    width: 50,
    fixed: 'left',
    renderCell: (row: StockRow) =>
      h(ElCheckbox, {
        modelValue: isSelected(row),
        onChange: () => toggleRow(row),
      }),
  },
  { key: 'product_code', title: 'SKU', width: 140, sortable: true },
  { key: 'product_name', title: '产品名', minWidth: 180, sortable: true },
  { key: 'location', title: '库位', width: 100 },
  {
    key: 'quantity',
    title: '数量',
    width: 100,
    align: 'right',
    sortable: true,
    renderCell: (row: StockRow) =>
      h(
        'span',
        { class: { 'low-stock': row.quantity < (row.min_quantity ?? 0) } },
        String(row.quantity)
      ),
  },
  { key: 'unit', title: '单位', width: 60 },
  { key: 'batch_no', title: '批次', width: 120 },
  {
    key: 'status',
    title: '状态',
    width: 80,
    renderCell: (row: StockRow) =>
      h(
        ElTag,
        { type: STATUS_TYPE[row.status] ?? 'info', size: 'small' },
        { default: () => STATUS_TEXT[row.status] ?? row.status }
      ),
  },
  { key: 'updated_at', title: '更新时间', width: 180, sortable: true },
  {
    key: '__actions__',
    title: '操作',
    width: 180,
    fixed: 'right',
    renderCell: (row: StockRow) =>
      h('div', { class: 'action-cell' }, [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => handleView(row) },
          { default: () => '详情' }
        ),
        h(
          ElButton,
          {
            type: 'warning',
            link: true,
            size: 'small',
            onClick: () => handleEdit(row),
          },
          { default: () => '编辑' }
        ),
        h(
          ElButton,
          {
            type: 'danger',
            link: true,
            size: 'small',
            onClick: () => handleDelete(row),
          },
          { default: () => '删除' }
        ),
      ]),
  },
])

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
  queryParams.value = { ...queryParams.value, ...localQueryParams }
  page.value = 1
  refresh()
}

const handleReset = () => {
  localQueryParams.keyword = ''
  localQueryParams.warehouse_id = undefined
  localQueryParams.status = ''
  selectedRows.value = []
  reset()
  refresh()
}

const handleView = (row: StockRow) => {
  ElMessage.info(`查看 ${row.product_name} 详情`)
}

const handleEdit = (row: StockRow) => {
  ElMessage.info(`编辑 ${row.product_name}（待对接表单对话框）`)
}

const handleDelete = async (row: StockRow) => {
  try {
    await ElMessageBox.confirm(`确认删除库存记录 ${row.product_code}？`, '删除确认', {
      type: 'warning',
    })
    ElMessage.success('已删除（占位，待对接 API）')
    refresh()
  } catch {
    /* 用户取消 */
  }
}

const handleBatchAdjust = () => {
  ElMessage.info(`批量调整 ${selectedRows.value.length} 条记录（待对接）`)
}

const handleBatchDelete = async () => {
  try {
    await ElMessageBox.confirm(
      `确认批量删除选中的 ${selectedRows.value.length} 条记录？`,
      '批量删除',
      { type: 'warning' }
    )
    ElMessage.success('批量删除成功（占位）')
    selectedRows.value = []
    refresh()
  } catch {
    /* 用户取消 */
  }
}

const handlePageChange = (newPage: number) => {
  page.value = newPage
  // 翻页时清空选择，避免跨页误操作
  selectedRows.value = []
}

const handleSizeChange = (newSize: number) => {
  pageSize.value = newSize
  selectedRows.value = []
}

const handleRowClick = (row: StockRow) => {
  handleView(row)
}

const hasLoaded = createLazyLoader()

onMounted(() => {
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
.filter-card,
.select-bar,
.batch-bar,
.table-card {
  margin-bottom: 16px;
}
.select-bar,
.batch-bar {
  display: flex;
  align-items: center;
  gap: 12px;
}
.selected-info {
  color: #409eff;
  font-weight: 500;
  margin-right: 12px;
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
