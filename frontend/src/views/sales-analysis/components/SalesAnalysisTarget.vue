<!--
  SalesAnalysisTarget.vue - 销售目标表（含完成率/差异/状态标签）
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="target-card">
    <template #header>
      <div class="card-header">
        <span>{{ t('salesAnalysis.target.cardTitle') }}</span>
        <el-button type="primary" size="small" @click="emit('edit-target')">
          <el-icon><Edit /></el-icon>
          {{ t('salesAnalysis.target.buttonEdit') }}
        </el-button>
      </div>
    </template>
    <el-table :data="data" border :aria-label="t('salesAnalysis.target.ariaLabelList')">
      <el-table-column
        prop="period"
        :label="t('salesAnalysis.target.columnPeriod')"
        width="120"
        align="center"
      />
      <el-table-column
        prop="target_amount"
        :label="t('salesAnalysis.target.columnTargetAmount')"
        width="150"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.target_amount) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="actual_amount"
        :label="t('salesAnalysis.target.columnActualAmount')"
        width="150"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.actual_amount) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="completion_rate"
        :label="t('salesAnalysis.target.columnCompletionRate')"
        width="120"
        align="center"
      >
        <template #default="{ row }">
          <el-progress
            :percentage="row.completion_rate"
            :color="getProgressColor(row.completion_rate)"
          />
        </template>
      </el-table-column>
      <el-table-column
        prop="variance"
        :label="t('salesAnalysis.target.columnVariance')"
        width="150"
        align="right"
      >
        <template #default="{ row }">
          <span :class="row.variance >= 0 ? 'text-success' : 'text-danger'">
            {{ row.variance >= 0 ? '+' : '' }}{{ formatCurrency(row.variance) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column
        prop="status"
        :label="t('salesAnalysis.target.columnStatus')"
        width="100"
        align="center"
      >
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
import { useI18n } from 'vue-i18n';
import { Edit } from '@element-plus/icons-vue';
import type { SalesTarget } from '@/api/sales-analysis';
import { formatCurrency, getProgressColor, getTargetStatusType } from '../composables/saFmts';

const { t } = useI18n({ useScope: 'global' });

defineProps<{ data: SalesTarget[] }>();
const emit = defineEmits<{ 'edit-target': [] }>();

/** 销售目标状态码 → i18n 标签（语言切换响应） */
const getTargetStatusLabel = (status: string): string => {
  switch (status) {
    case 'COMPLETED':
      return t('salesAnalysis.target.statusCompleted');
    case 'IN_PROGRESS':
      return t('salesAnalysis.target.statusInProgress');
    case 'PARTIAL':
      return t('salesAnalysis.target.statusPartial');
    case 'NOT_STARTED':
      return t('salesAnalysis.target.statusNotStarted');
    default:
      return status;
  }
};
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
