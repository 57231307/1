<!--
  SaTarget.vue - 销售目标表（含完成率/差异/状态标签）
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="target-card">
    <template #header>
      <div class="card-header">
        <span>销售目标</span>
        <el-button type="primary" size="small" @click="emit('edit-target')">
          <el-icon><Edit /></el-icon>
          编辑目标
        </el-button>
      </div>
    </template>
    <el-table :data="data" border>
      <el-table-column prop="period" label="周期" width="120" align="center" />
      <el-table-column prop="target_amount" label="目标金额" width="150" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.target_amount) }}
        </template>
      </el-table-column>
      <el-table-column prop="actual_amount" label="实际金额" width="150" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.actual_amount) }}
        </template>
      </el-table-column>
      <el-table-column prop="completion_rate" label="完成率" width="120" align="center">
        <template #default="{ row }">
          <el-progress
            :percentage="row.completion_rate"
            :color="getProgressColor(row.completion_rate)"
          />
        </template>
      </el-table-column>
      <el-table-column prop="variance" label="差异" width="150" align="right">
        <template #default="{ row }">
          <span :class="row.variance >= 0 ? 'text-success' : 'text-danger'">
            {{ row.variance >= 0 ? '+' : '' }}{{ formatCurrency(row.variance) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getTargetStatusType(row.status)">{{
            getTargetStatusLabel(row.status)
          }}</el-tag>
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { Edit } from '@element-plus/icons-vue'
import type { SalesTarget } from '@/api/sales-analysis'
import {
  formatCurrency,
  getProgressColor,
  getTargetStatusType,
  getTargetStatusLabel,
} from '../composables/saFmts'

defineProps<{ data: SalesTarget[] }>()
const emit = defineEmits<{ 'edit-target': [] }>()
</script>

<style scoped>
.target-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.text-success {
  color: #52c41a;
}

.text-danger {
  color: #f5222d;
}
</style>
