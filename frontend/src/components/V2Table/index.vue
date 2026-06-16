<script setup lang="ts">
import { computed } from 'vue'
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
    cellRenderer: ({ rowData }: any) => {
      const value = col.formatter ? col.formatter(rowData[col.key], rowData) : rowData[col.key]
      return value
    }
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
