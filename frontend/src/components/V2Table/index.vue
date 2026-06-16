<script setup lang="ts">
import { computed, ref } from 'vue'
import { ElTableV2, ElAutoResizer } from 'element-plus'

export interface ColumnDef {
  key: string
  label: string
  width: number
  formatter?: (value: any, row: any) => any
  align?: 'left' | 'center' | 'right'
  fixed?: 'left' | 'right'
  sortable?: boolean
}

interface Props {
  data: any[]
  columns: ColumnDef[]
  estimatedRowHeight?: number
  loading?: boolean
  emptyText?: string
}

const props = withDefaults(defineProps<Props>(), {
  estimatedRowHeight: 48,
  loading: false,
  emptyText: '暂无数据'
})

const emit = defineEmits<{
  (e: 'row-click', row: any, column: any, event: Event): void
  (e: 'selection-change', selection: any[]): void
}>()

// renderCell 计数器（暴露到 window 供性能测试采集）
const renderCellCount = ref(0)

if (typeof window !== 'undefined') {
  ;(window as any).__renderCellTotal = renderCellCount
}

// renderCell WeakMap 缓存
const cellCache = new WeakMap<object, Map<string, any>>()

function getCachedCell(row: any, col: ColumnDef): any {
  let rowCache = cellCache.get(row)
  if (!rowCache) {
    rowCache = new Map()
    cellCache.set(row, rowCache)
  }
  if (rowCache.has(col.key)) return rowCache.get(col.key)
  // 未命中：计数 + 计算 + 缓存
  renderCellCount.value++
  const value = col.formatter ? col.formatter(row[col.key], row) : row[col.key]
  rowCache.set(col.key, value)
  return value
}

// 列定义转 el-table-v2 columns
const v2Columns = computed(() =>
  props.columns.map(col => ({
    key: col.key,
    title: col.label,
    dataKey: col.key,
    width: col.width,
    align: col.align,
    fixed: col.fixed,
    sortable: col.sortable,
    cellRenderer: ({ rowData }: any) => getCachedCell(rowData, col)
  }))
)
</script>

<template>
  <ElAutoResizer>
    <template #default="{ height: autoHeight, width }">
      <ElTableV2
        :columns="v2Columns"
        :data="data"
        :width="width"
        :height="autoHeight"
        :estimated-row-height="estimatedRowHeight"
        :loading="loading"
        :empty-text="emptyText"
        :row-key="(row: any) => row.id"
        @row-click="(e: any) => emit('row-click', e.rowData, e.column, e.event)"
        @selection-change="(e: any) => emit('selection-change', e)"
      />
    </template>
  </ElAutoResizer>
</template>
