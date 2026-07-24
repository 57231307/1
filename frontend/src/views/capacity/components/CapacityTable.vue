<!--
  CapacityTable.vue - 工作中心列表表格（带分页 + 负荷率/状态/瓶颈）
  拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>工作中心列表</span>
        <el-button type="primary" link @click="emit('refresh')">
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </template>
    <el-table v-loading="tableLoading" :data="data" stripe style="width: 100%" aria-label="工作中心列表">
      <el-table-column prop="code" label="编号" width="120" />
      <el-table-column prop="name" label="名称" width="150" />
      <el-table-column prop="capacity_hours" label="产能工时" width="120" />
      <el-table-column prop="used_hours" label="已用工时" width="120" />
      <el-table-column prop="load_rate" label="负荷率" width="120">
        <template #default="{ row }">
          <el-tag :type="getLoadRateType(row.load_rate)">{{
            (row.load_rate * 100).toFixed(1)
          }}%</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="bottleneck" label="瓶颈" width="80">
        <template #default="{ row }">
          <el-tag v-if="row.bottleneck" type="danger" size="small">是</el-tag>
          <span v-else>-</span>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      :current-page="page"
      :page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50]"
      layout="total, sizes, prev, pager, next"
      class="pagination"
      aria-label="工作中心列表分页"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { Refresh } from '@element-plus/icons-vue'
import type { WorkCenter } from '@/api/capacity'
import { getStatusType, getStatusLabel, getLoadRateType } from '../composables/cpFmts'

defineProps<{
  data: WorkCenter[]
  tableLoading: boolean
  total: number
  page: number
  pageSize: number
}>()

const emit = defineEmits<{
  refresh: []
  'update:page': [v: number]
  'update:size': [v: number]
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header span {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
</style>
