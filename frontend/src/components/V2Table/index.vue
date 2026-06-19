<!--
  V2Table - 基于 el-table-v2 的通用虚拟滚动表格组件

  设计要点（Wave 4 P2-3 重做）：
  - 对齐 P2-1（test 分支）API：title/width?/formatter(row) → string/renderCell(row) 钩子
  - 保留 P2-3 性能价值：
    · WeakMap 缓存（cellCache + getCachedCell）避免 cellRenderer 重复计算
    · renderCell 计数器（暴露到 window.__renderCellTotal 供性能测试采集）
    · estimatedRowHeight prop（页面级行高调优，inventory=40/sales=56/production=48/quality=44）
  - 新增 P2-1 特性：内置分页（page/pageSize/total/pageSizes + @page-change/@size-change）
-->
<template>
  <div
    class="v2-table-wrapper"
    :style="{ height: typeof height === 'number' ? `${height}px` : height || '600px' }"
  >
    <ElAutoResizer>
      <template #default="{ height: autoHeight, width }">
        <ElTableV2
          :columns="v2Columns"
          :data="data"
          :width="width"
          :height="autoHeight"
          :row-key="rowKey"
          :loading="loading"
          :empty-text="emptyText"
          :estimated-row-height="estimatedRowHeight"
          :header-height="48"
          fixed
          @row-click="handleRowClick"
        />
      </template>
    </ElAutoResizer>
    <div v-if="total !== undefined" class="v2-table-pagination">
      <ElPagination
        :current-page="page"
        :page-size="pageSize"
        :total="total"
        :page-sizes="pageSizes"
        layout="total, sizes, prev, pager, next, jumper"
        @current-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * V2Table 组件
 * 包装 el-table-v2，统一列定义 / 虚拟滚动 / 分页 / 事件接口
 */
import { computed, ref } from 'vue'
import { ElAutoResizer, ElTableV2, ElPagination } from 'element-plus'
import type { CellRendererParams } from 'element-plus/es/components/table-v2/src/types'
import type { ColumnDef, SortOrder } from './types'

const props = withDefaults(
  defineProps<{
    columns: ColumnDef[]
    data: any[]
    loading?: boolean
    total?: number
    page?: number
    pageSize?: number
    pageSizes?: number[]
    height?: number | string
    rowKey?: string
    emptyText?: string
    /** P2-3 价值：页面级行高调优（默认 48） */
    estimatedRowHeight?: number
  }>(),
  {
    loading: false,
    pageSizes: () => [10, 20, 50, 100],
    height: 600,
    rowKey: 'id',
    emptyText: '暂无数据',
    estimatedRowHeight: 48,
  }
)

const emit = defineEmits<{
  'page-change': [page: number]
  'size-change': [size: number]
  'sort-change': [key: string, order: SortOrder]
  'row-click': [row: any]
  refresh: []
}>()

/**
 * renderCell 计数器（暴露到 window 供性能测试采集）
 * 来源：P2-3 价值保留（b1ae10f perf(V2Table): renderCell 计数器暴露 window 供性能测试）
 */
const renderCellCount = ref(0)
if (typeof window !== 'undefined') {
  ;(window as any).__renderCellTotal = renderCellCount
}

/**
 * renderCell WeakMap 缓存
 * 缓存结构：cellCache[row][col.key] → 已渲染的 VNode/string
 * 来源：P2-3 价值保留（8aebbb4 perf(V2Table): renderCell WeakMap 缓存避免重复计算）
 */
const cellCache = new WeakMap<object, Map<string, any>>()

/**
 * 获取单行单列的缓存值，未命中则计算并写入
 */
function getCachedCell(row: any, col: ColumnDef): any {
  let rowCache = cellCache.get(row)
  if (!rowCache) {
    rowCache = new Map()
    cellCache.set(row, rowCache)
  }
  if (rowCache.has(col.key)) {
    return rowCache.get(col.key)
  }
  // 未命中：递增计数 + 计算 + 缓存
  renderCellCount.value++
  let value: any
  if (col.renderCell) {
    // renderCell 优先级最高：返回 VNode
    value = col.renderCell(row)
  } else if (col.formatter) {
    // formatter(row) 签名（P2-1 风格）
    value = col.formatter(row)
  } else {
    // 兜底：直接显示值
    const v = row[col.key]
    value = v !== null && v !== undefined ? String(v) : ''
  }
  rowCache.set(col.key, value)
  return value
}

/**
 * 将 ColumnDef 转换为 el-table-v2 接受的列配置
 * 关键差异：title 来源于 col.title（非旧 label）、width 可选（默认 150）
 * 注：el-table-v2 的 fixed 字段是 `true | FixedDir` (const enum)，与 string literal 不直接兼容
 */
const v2Columns = computed(
  () =>
    props.columns
      .filter(col => !col.hidden)
      .map(col => ({
        key: col.key,
        title: col.title,
        dataKey: col.key,
        width: col.width ?? 150,
        minWidth: col.minWidth,
        fixed: col.fixed,
        sortable: col.sortable,
        align: col.align ?? 'left',
        cellRenderer: (params: { rowData: any; rowIndex: number }) =>
          getCachedCell(params.rowData, col),
      })) as any
)

const handlePageChange = (newPage: number) => {
  emit('page-change', newPage)
}

const handleSizeChange = (newSize: number) => {
  emit('size-change', newSize)
}

const handleRowClick = ({ rowData }: { rowData: any }) => {
  emit('row-click', rowData)
}
</script>

<style scoped>
.v2-table-wrapper {
  display: flex;
  flex-direction: column;
  width: 100%;
}
.v2-table-pagination {
  display: flex;
  justify-content: flex-end;
  padding: 16px 0;
}
</style>
