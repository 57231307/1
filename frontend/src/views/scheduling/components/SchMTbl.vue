<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchMTbl.vue - 排产工单列表
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>排程工单列表</span>
        <div class="header-ops">
          <el-select
            :model-value="filterStatus"
            placeholder="筛选状态"
            clearable
            style="width: 140px; margin-right: 8px"
            @update:model-value="onFilterChange"
            @change="emit('filter-change')"
          >
            <el-option label="全部" value="" />
            <el-option label="待排程" value="pending" />
            <el-option label="已排程" value="scheduled" />
            <el-option label="生产中" value="running" />
            <el-option label="已完成" value="completed" />
            <el-option label="冲突" value="conflict" />
          </el-select>
          <el-button type="primary" link @click="emit('refresh')">
            <el-icon><Refresh /></el-icon>
            刷新
          </el-button>
        </div>
      </div>
    </template>
    <el-table v-loading="taskLoading" :data="taskList" stripe>
      <el-table-column prop="order_no" label="工单号" width="140" />
      <el-table-column prop="product_name" label="产品名称" width="160" />
      <el-table-column prop="work_center_name" label="工作中心" width="130" />
      <el-table-column prop="quantity" label="数量" width="80" />
      <el-table-column label="开始时间" width="170">
        <template #default="{ row }">{{ formatDateTime(row.start_time) }}</template>
      </el-table-column>
      <el-table-column label="结束时间" width="170">
        <template #default="{ row }">{{ formatDateTime(row.end_time) }}</template>
      </el-table-column>
      <el-table-column prop="duration_hours" label="时长(h)" width="80" />
      <el-table-column label="优先级" width="90">
        <template #default="{ row }">
          <el-tag :type="getPriorityType(row.priority)" size="small">P{{ row.priority }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)" effect="light">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" fixed="right" width="160">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('adjust', row)">调整</el-button>
          <el-button
            v-if="row.has_conflict"
            type="danger"
            link
            size="small"
            @click="emit('conflict-detail', row)"
          >
            详情
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      :current-page="currentPage"
      :page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50]"
      layout="total, sizes, prev, pager, next"
      class="pagination"
      @update:current-page="onPageChange"
      @update:page-size="onSizeChange"
      @size-change="emit('size-change')"
      @current-change="emit('current-change')"
    />
  </el-card>
</template>

<script setup lang="ts">
import type { ScheduleTask } from '@/api/scheduling'
import { formatDateTime, getStatusType, getStatusLabel, getPriorityType } from '../composables/schMFmts'

// 排产工单列表属性
defineProps<{
  // 工单列表
  taskList: ScheduleTask[]
  // 加载状态
  taskLoading: boolean
  // 总数
  total: number
  // 当前页
  currentPage: number
  // 每页大小
  pageSize: number
  // 筛选状态
  filterStatus: string
}>()

// 定义事件
const emit = defineEmits<{
  // 调整
  (e: 'adjust', row: ScheduleTask): void
  // 冲突详情
  (e: 'conflict-detail', row: ScheduleTask): void
  // 刷新
  (e: 'refresh'): void
  // 筛选变化
  (e: 'filter-change'): void
  // 筛选值变化
  (e: 'update:filterStatus', value: string): void
  // 当前页变化
  (e: 'update:currentPage', value: number): void
  // 每页大小变化
  (e: 'update:pageSize', value: number): void
  // 分页 - 每页大小
  (e: 'size-change'): void
  // 分页 - 当前页
  (e: 'current-change'): void
}>()

/** 筛选值变化 */
const onFilterChange = (v: string) => {
  emit('update:filterStatus', v)
}

/** 当前页变化 */
const onPageChange = (v: number) => {
  emit('update:currentPage', v)
}

/** 每页大小变化 */
const onSizeChange = (v: number) => {
  emit('update:pageSize', v)
}
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

.header-ops {
  display: flex;
  align-items: center;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
</style>
