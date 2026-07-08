<!--
  V2Table - 基于 el-table-v2 的通用虚拟滚动表格组件

  设计要点：
  - 对齐友好 API：title/width?/formatter(row) → string/renderCell(row) 钩子
  - 内置分页（page/pageSize/total/pageSizes + @page-change/@size-change）
  - 行点击事件通过 el-table-v2 官方 rowEventHandlers prop 接入（非 @row-click）
  - 列定义通过 ColumnDef<T> 泛型由调用方显式指定行数据类型
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
          :row-event-handlers="rowEventHandlers"
          fixed
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

<script setup lang="ts" generic="T">
/**
 * V2Table 组件
 * 包装 el-table-v2，统一列定义 / 虚拟滚动 / 分页 / 事件接口
 */
import { computed, h } from 'vue'
import { ElAutoResizer, ElTableV2, ElPagination, TableV2FixedDir } from 'element-plus'
import type { Column, RowEventHandlerParams } from 'element-plus'
import type { ColumnDef, SortOrder } from './types'

const props = withDefaults(
  defineProps<{
    columns: ColumnDef<T>[]
    data: T[]
    loading?: boolean
    total?: number
    page?: number
    pageSize?: number
    pageSizes?: number[]
    height?: number | string
    rowKey?: string
    emptyText?: string
    /// 页面级行高调优（默认 48）
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
  'row-click': [row: T]
  refresh: []
}>()

/// 将 ColumnDef 转换为 el-table-v2 官方 Column 配置（cellRenderer 参数类型由官方自动推导）
const v2Columns = computed<Column<T>[]>(() =>
  props.columns
    .filter(col => !col.hidden)
    .map((col): Column<T> => {
      const column: Column<T> = {
        key: col.key,
        title: col.title,
        dataKey: col.key,
        width: col.width ?? 150,
        minWidth: col.minWidth,
        /// fixed 字段：element-plus 官方 FixedDir 是 enum（非字符串字面量），需映射
        fixed: col.fixed === 'left' ? TableV2FixedDir.LEFT : col.fixed === 'right' ? TableV2FixedDir.RIGHT : undefined,
        sortable: col.sortable,
        align: col.align ?? 'left',
      }
      /// 有自定义渲染或格式化时，提供 cellRenderer；否则由 el-table-v2 用 dataKey 自动渲染
      if (col.renderCell || col.formatter) {
        column.cellRenderer = (params) => {
          /// 官方 RowCommonParams.rowData 为 any，收窄为 T（element-plus 类型系统设计）
          const row: T = params.rowData
          if (col.renderCell) {
            return col.renderCell(row)
          }
          if (col.formatter) {
            return h('span', col.formatter(row))
          }
          return h('span', '')
        }
      }
      return column
    })
)

/// 行事件处理器：通过 el-table-v2 官方 rowEventHandlers prop 接入点击事件
const rowEventHandlers = {
  onClick: (params: RowEventHandlerParams) => {
    /// 官方 RowCommonParams.rowData 为 any，收窄为 T
    const row: T = params.rowData
    emit('row-click', row)
  },
}

const handlePageChange = (newPage: number) => {
  emit('page-change', newPage)
}

const handleSizeChange = (newSize: number) => {
  emit('size-change', newSize)
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
