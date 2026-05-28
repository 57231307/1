<template>
  <div class="draggable-table-demo">
    <el-card>
      <template #header>拖拽排序表格示例</template>

      <DraggableTable
        v-model:data="tableData"
        :show-selection="true"
        :show-drag-handle="true"
        :show-index="true"
        border
        stripe
        @sort-change="handleSortChange"
        @selection-change="handleSelectionChange"
      >
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="name" label="名称" />
        <el-table-column prop="category" label="分类" width="120" />
        <el-table-column prop="price" label="价格" width="120">
          <template #default="{ row }"> ¥{{ row.price.toFixed(2) }} </template>
        </el-table-column>
        <el-table-column prop="sort" label="排序" width="80" />
      </DraggableTable>

      <el-alert
        title="提示：拖拽左侧排序图标可调整行顺序"
        type="info"
        :closable="false"
        show-icon
        style="margin-top: 16px"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import DraggableTable from '@/components/DraggableTable.vue'

const tableData = ref([
  { id: 1, name: '面料 A1', category: '棉布', price: 25.5, sort: 1 },
  { id: 2, name: '面料 B2', category: '丝绸', price: 88.0, sort: 2 },
  { id: 3, name: '面料 C3', category: '麻布', price: 42.3, sort: 3 },
  { id: 4, name: '面料 D4', category: '化纤', price: 15.8, sort: 4 },
  { id: 5, name: '面料 E5', category: '混纺', price: 55.6, sort: 5 },
  { id: 6, name: '面料 F6', category: '棉布', price: 30.0, sort: 6 },
])

const handleSortChange = (data: any[]) => {
  console.log(
    '排序变更:',
    data.map((item, index) => ({
      id: item.id,
      newSort: index + 1,
    }))
  )
}

const handleSelectionChange = (selection: any[]) => {
  console.log('选中项:', selection)
}
</script>

<style scoped>
.draggable-table-demo {
  padding: 10px;
}
</style>
