<!--
  V2Table - 基于 el-table-v2 的通用虚拟滚动表格组件
  任务编号: Wave 4 P2-1 PR-1
  关联 spec: docs/superpowers/specs/2026-06-16-wave4-p2-1-design.md
-->
<template>
  <div class="v2-table-wrapper" :style="{ height: typeof height === 'number' ? `${height}px` : height || '600px' }">
    <el-auto-resizer>
      <template #default="{ height: autoHeight, width }">
        <el-table-v2
          :columns="v2Columns"
          :data="data"
          :width="width"
          :height="autoHeight"
          :row-key="rowKey"
          :loading="loading"
          :empty-text="emptyText"
          :estimated-row-height="48"
          :header-height="48"
          fixed
          @row-click="handleRowClick"
        />
      </template>
    </el-auto-resizer>
    <div v-if="total !== undefined" class="v2-table-pagination">
      <el-pagination
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
import { computed, h } from 'vue'
import { ElAutoResizer, ElTableV2, ElPagination } from 'element-plus'
import type { Column as V2Column, CellRendererParams } from 'element-plus/es/components/table-v2/src/types'
import type { ColumnDef, SortOrder } from './types'

const props = withDefaults(defineProps<{
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
}>(), {
  loading: false,
  pageSizes: () => [10, 20, 50, 100],
  height: 600,
  rowKey: 'id',
  emptyText: '暂无数据',
})

const emit = defineEmits<{
  'page-change': [page: number]
  'size-change': [size: number]
  'sort-change': [key: string, order: SortOrder]
  'row-click': [row: any]
  'refresh': []
}>()

/**
 * 将 ColumnDef 转换为 el-table-v2 接受的列配置
 * 使用 V2Column<any> 显式标注计算属性的返回类型，
 * 避免推断类型与 el-table-v2 columns 期望不匹配（TS2322 AnyColumn）
 */
const v2Columns = computed<V2Column<any>[]>(() => {
  return props.columns
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
      // cellRenderer 必须接受完整 CellRendererParams<any>（参数协变校验）
      cellRenderer: (params: CellRendererParams<any>) => {
        const row = params.rowData
        if (col.renderCell) {
          return col.renderCell(row)
        }
        const value = row[col.key]
        if (col.formatter) {
          return h('span', col.formatter(row))
        }
        return h('span', value !== null && value !== undefined ? String(value) : '')
      },
    }))
})

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
