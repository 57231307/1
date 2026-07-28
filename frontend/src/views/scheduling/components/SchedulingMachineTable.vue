<!--
  SchedulingMachineTable.vue - 排产主页工单列表
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>{{ t('scheduling.machineTable.title') }}</span>
        <div class="header-ops">
          <el-select
            :model-value="filterStatus"
            :placeholder="t('scheduling.machineTable.placeholder.filterStatus')"
            clearable
            style="width: 140px; margin-right: 8px"
            @update:model-value="onFilterChange"
            @change="emit('filter-change')"
          >
            <el-option :label="t('scheduling.machineTable.filterOption.all')" value="" />
            <el-option :label="t('scheduling.machineTable.filterOption.pending')" value="pending" />
            <el-option
              :label="t('scheduling.machineTable.filterOption.scheduled')"
              value="scheduled"
            />
            <el-option :label="t('scheduling.machineTable.filterOption.running')" value="running" />
            <el-option
              :label="t('scheduling.machineTable.filterOption.completed')"
              value="completed"
            />
            <el-option
              :label="t('scheduling.machineTable.filterOption.conflict')"
              value="conflict"
            />
          </el-select>
          <el-button type="primary" link @click="emit('refresh')">
            <el-icon><Refresh /></el-icon>
            {{ t('scheduling.machineTable.button.refresh') }}
          </el-button>
        </div>
      </div>
    </template>
    <el-table
      v-loading="taskLoading"
      :data="taskList"
      stripe
      :aria-label="t('scheduling.machineTable.ariaLabel.table')"
    >
      <el-table-column
        prop="order_no"
        :label="t('scheduling.machineTable.column.orderNo')"
        width="140"
      />
      <el-table-column
        prop="product_name"
        :label="t('scheduling.machineTable.column.productName')"
        width="160"
      />
      <el-table-column
        prop="work_center_name"
        :label="t('scheduling.machineTable.column.workCenter')"
        width="130"
      />
      <el-table-column
        prop="quantity"
        :label="t('scheduling.machineTable.column.quantity')"
        width="80"
      />
      <el-table-column :label="t('scheduling.machineTable.column.startTime')" width="170">
        <template #default="{ row }">{{ formatDateTime(row.start_time) }}</template>
      </el-table-column>
      <el-table-column :label="t('scheduling.machineTable.column.endTime')" width="170">
        <template #default="{ row }">{{ formatDateTime(row.end_time) }}</template>
      </el-table-column>
      <el-table-column
        prop="duration_hours"
        :label="t('scheduling.machineTable.column.duration')"
        width="80"
      />
      <el-table-column :label="t('scheduling.machineTable.column.priority')" width="90">
        <template #default="{ row }">
          <el-tag :type="getPriorityType(row.priority)" size="small">P{{ row.priority }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('scheduling.machineTable.column.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)" effect="light">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        :label="t('scheduling.machineTable.column.operation')"
        fixed="right"
        width="160"
      >
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('adjust', row)">{{
            t('scheduling.machineTable.button.adjust')
          }}</el-button>
          <el-button
            v-if="row.has_conflict"
            type="danger"
            link
            size="small"
            @click="emit('conflict-detail', row)"
          >
            {{ t('scheduling.machineTable.button.detail') }}
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
      :aria-label="t('scheduling.machineTable.ariaLabel.pagination')"
      @update:current-page="(v: number) => emit('update:currentPage', v)"
      @update:page-size="(v: number) => emit('update:pageSize', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { ScheduleTask } from '@/api/scheduling';
import { Refresh } from '@element-plus/icons-vue';
import { formatDateTime, getStatusType, getPriorityType } from '../composables/schMFmts';

const { t } = useI18n({ useScope: 'global' });

/** 状态标签映射（响应式 i18n，覆盖 schMFmts 静态版本） */
const getStatusLabel = (status: string): string => t(`scheduling.machineTable.status.${status}`);

// 排产工单列表属性
defineProps<{
  // 工单列表
  taskList: ScheduleTask[];
  // 加载状态
  taskLoading: boolean;
  // 总数
  total: number;
  // 当前页
  currentPage: number;
  // 每页大小
  pageSize: number;
  // 筛选状态
  filterStatus: string;
}>();

// 定义事件（object 形式，Vue 3.3+ 语法，与 I-3 第 1 批保持一致）
const emit = defineEmits<{
  // 调整
  adjust: [row: ScheduleTask];
  // 冲突详情
  'conflict-detail': [row: ScheduleTask];
  // 刷新
  refresh: [];
  // 筛选变化
  'filter-change': [];
  // 筛选值变化
  'update:filterStatus': [value: string];
  // 当前页变化
  'update:currentPage': [value: number];
  // 每页大小变化
  'update:pageSize': [value: number];
}>();

/** 筛选值变化 */
const onFilterChange = (v: string) => {
  emit('update:filterStatus', v);
};
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
