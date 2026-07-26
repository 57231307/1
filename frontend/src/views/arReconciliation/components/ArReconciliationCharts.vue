<!--
  ArReconciliationCharts.vue - AR 对账账龄图表（柱状 + 饼图）
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="chart-row">
    <el-col :span="12">
      <el-card shadow="hover" :aria-label="t('arReconciliationModule.charts.barChartAriaLabel')">
        <div ref="chartRef" class="chart-container" style="height: 320px"></div>
      </el-card>
    </el-col>
    <el-col :span="12">
      <el-card shadow="hover" :aria-label="t('arReconciliationModule.charts.pieChartAriaLabel')">
        <div ref="pieChartRef" class="chart-container" style="height: 320px"></div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';

/**
 * 账龄图表组件
 * 接收父组件传入的 ECharts 实例 ref 进行图表渲染
 */
const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  /** 柱状图 DOM ref */
  chartRef: { value: HTMLDivElement | null };
  /** 饼图 DOM ref */
  pieChartRef: { value: HTMLDivElement | null };
}>();

void props;
</script>

<style scoped>
.chart-row {
  margin-bottom: 20px;
}
.chart-container {
  width: 100%;
}
</style>
