<template>
  <div class="draggable-table">
    <el-table
      ref="tableRef"
      v-bind="$attrs"
      :data="tableData"
      row-key="id"
      @selection-change="handleSelectionChange"
    >
      <el-table-column v-if="showSelection" type="selection" width="55" fixed="left" />

      <el-table-column v-if="showDragHandle" width="60" fixed="left" align="center">
        <template #header>
          <el-tooltip content="拖拽排序" placement="top">
            <el-icon><Rank /></el-icon>
          </el-tooltip>
        </template>
        <template #default>
          <el-icon class="drag-handle">
            <Rank />
          </el-icon>
        </template>
      </el-table-column>

      <slot />

      <el-table-column v-if="showIndex" type="index" label="序号" width="60" fixed="left" />
    </el-table>

    <div v-if="showPagination" class="pagination-wrapper">
      <el-pagination
        v-model:current-page="currentPage"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="pageSizes"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="handleSizeChange"
        @current-change="handleCurrentChange"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { Rank } from '@element-plus/icons-vue'
import Sortable from 'sortablejs'

interface Props {
  data: any[]
  loading?: boolean
  showSelection?: boolean
  showDragHandle?: boolean
  showIndex?: boolean
  showPagination?: boolean
  total?: number
  currentPage?: number
  pageSize?: number
  pageSizes?: number[]
  autoSave?: boolean
  dragHandleSelector?: string
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  showSelection: false,
  showDragHandle: true,
  showIndex: false,
  showPagination: false,
  total: 0,
  currentPage: 1,
  pageSize: 10,
  pageSizes: () => [10, 20, 50, 100],
  autoSave: true,
  dragHandleSelector: '.drag-handle'
})

const emit = defineEmits<{
  'update:data': [data: any[]]
  'update:currentPage': [page: number]
  'update:pageSize': [size: number]
  selectionChange: [selection: any[]]
  sortChange: [data: any[]]
  sizeChange: [size: number]
  currentChange: [page: number]
}>()

const tableRef = ref()
const selectedRows = ref<any[]>([])

const tableData = computed({
  get: () => props.data,
  set: (val) => emit('update:data', val)
})

const currentPage = computed({
  get: () => props.currentPage,
  set: (val) => emit('update:currentPage', val)
})

const pageSize = computed({
  get: () => props.pageSize,
  set: (val) => emit('update:pageSize', val)
})

const handleSelectionChange = (selection: any[]) => {
  selectedRows.value = selection
  emit('selectionChange', selection)
}

const handleSizeChange = (size: number) => {
  emit('sizeChange', size)
}

const handleCurrentChange = (page: number) => {
  emit('currentChange', page)
}

const initSortable = () => {
  if (!tableRef.value) return

  const tbody = tableRef.value.$el.querySelector('.el-table__body-wrapper tbody')
  if (!tbody) return

  Sortable.create(tbody, {
    handle: props.dragHandleSelector,
    animation: 150,
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    onEnd: async (evt: any) => {
      const { oldIndex, newIndex } = evt
      if (oldIndex === newIndex || oldIndex === undefined || newIndex === undefined) return

      const newData = [...tableData.value]
      const movedItem = newData.splice(oldIndex, 1)[0]
      newData.splice(newIndex, 0, movedItem)

      emit('update:data', newData)
      emit('sortChange', newData)

      if (props.autoSave) {
        ElMessage.success('排序已保存')
      }
    }
  })
}

onMounted(async () => {
  await nextTick()
  initSortable()
})

defineExpose({
  tableRef,
  selectedRows,
  clearSelection: () => tableRef.value?.clearSelection(),
  toggleRowSelection: (row: any, selected?: boolean) => tableRef.value?.toggleRowSelection(row, selected)
})
</script>

<style scoped>
.draggable-table {
  position: relative;
}

.drag-handle {
  cursor: move;
  color: #909399;
  font-size: 16px;
  transition: color 0.3s;
}

.drag-handle:hover {
  color: #409eff;
}

.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  padding: 16px 0;
}

:deep(.el-table__row) {
  cursor: default;
}

:deep(.sortable-ghost) {
  opacity: 0.5;
  background: #f5f7fa;
}

:deep(.sortable-chosen) {
  background: #ecf5ff;
}
</style>
