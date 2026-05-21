<template>
  <div class="batch-actions-demo">
    <el-card>
      <template #header>批量操作组件示例</template>

      <BatchActions
        :selected-rows="selectedRows"
        @clear="selectedRows = []"
        @complete="handleComplete"
      />

      <el-table
        :data="tableData"
        @selection-change="handleSelectionChange"
        row-key="id"
        border
      >
        <el-table-column type="selection" width="55" />
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="name" label="名称" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'pending' ? 'warning' : 'success'">
              {{ row.status === 'pending' ? '待审批' : '已通过' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="date" label="创建日期" width="150" />
      </el-table>

      <el-alert
        v-if="selectedRows.length === 0"
        title="请勾选表格中的数据进行批量操作"
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
import BatchActions from '@/components/BatchActions.vue'
import { ElMessage as _ElMessage } from 'element-plus'

const selectedRows = ref<any[]>([])

const tableData = ref([
  { id: 1, name: '订单 #1001', status: 'pending', date: '2026-01-15' },
  { id: 2, name: '订单 #1002', status: 'pending', date: '2026-01-16' },
  { id: 3, name: '订单 #1003', status: 'approved', date: '2026-01-17' },
  { id: 4, name: '订单 #1004', status: 'pending', date: '2026-01-18' },
  { id: 5, name: '订单 #1005', status: 'pending', date: '2026-01-19' },
  { id: 6, name: '订单 #1006', status: 'approved', date: '2026-01-20' }
])

const handleSelectionChange = (selection: any[]) => {
  selectedRows.value = selection
}

const handleComplete = (key: string, success: boolean) => {
  if (success) {
    console.log(`操作 ${key} 完成`)
  }
}
</script>

<style scoped>
.batch-actions-demo {
  padding: 10px;
}
</style>
