<script setup lang="ts">
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

withDefaults(defineProps<Props>(), {
  estimatedRowHeight: 48,
  loading: false,
  emptyText: '暂无数据'
})
</script>

<template>
  <ElAutoResizer>
    <template #default="{ height: autoHeight, width }">
      <ElTableV2
        :columns="[]"
        :data="data"
        :width="width"
        :height="autoHeight"
        :estimated-row-height="estimatedRowHeight"
        :loading="loading"
        :empty-text="emptyText"
      />
    </template>
  </ElAutoResizer>
</template>
