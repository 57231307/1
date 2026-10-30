<!--
  SalesAnalysisTarget.vue - 销售目标表（含完成率、差额、状态标签列）
  出参派生列可空，后端契约对齐 api/sales-analysis.ts 的 SalesTarget。
  null（目标未设或实际额无法判定）走公共缺值占位，不渲染进度条、不渲染百分号、
  不渲染货币符号、不合并成 0；着色与比较分支前显式判 null。
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
          <!-- completion_rate 为 null 含"目标未设"与"实际额无法判定"两态（由目标列与
               实际列区分），此处一律走缺值占位；不渲染进度条、不渲染百分号、不以 0 兜底 -->
          <el-progress
            v-if="row.completion_rate !== null"
            :percentage="Number(row.completion_rate)"
            :color="getProgressColor(row.completion_rate)"
          />
          <span v-else>{{ t('common.valueMissing') }}</span>
        </template>
      </el-table-column>
      <el-table-column
        prop="variance"
        :label="t('salesAnalysis.target.columnVariance')"
        width="150"
        align="right"
      >
        <template #default="{ row }">
          <!-- variance 仅在 actual_amount 为 null 时为 null；着色与正负号分支前显式判 null，
               不依赖 Number(null) 为 0 的隐式转换 -->
          <span v-if="row.variance === null">{{ t('common.valueMissing') }}</span>
          <span v-else :class="Number(row.variance) >= 0 ? 'text-success' : 'text-danger'">
            {{ Number(row.variance) >= 0 ? '+' : '' }}{{ formatCurrency(row.variance) }}
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
