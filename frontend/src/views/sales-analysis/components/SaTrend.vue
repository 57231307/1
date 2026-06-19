<!--
  SaTrend.vue - 销售趋势 + 销售构成占位面板
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="chart-row">
    <el-col :xs="24" :lg="16">
      <el-card shadow="hover" class="chart-card">
        <template #header>
          <div class="card-header">
            <span>销售趋势</span>
            <el-radio-group :model-value="trendPeriod" size="small" @update:model-value="updatePeriod">
              <el-radio-button label="week">本周</el-radio-button>
              <el-radio-button label="month">本月</el-radio-button>
              <el-radio-button label="quarter">本季度</el-radio-button>
              <el-radio-button label="year">本年</el-radio-button>
            </el-radio-group>
          </div>
        </template>
        <div class="chart-container">
          <!-- 趋势图表占位 -->
          <div class="chart-placeholder">
            <el-icon><TrendCharts /></el-icon>
            <p>销售趋势图表</p>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :lg="8">
      <el-card shadow="hover" class="chart-card">
        <template #header>
          <span>销售构成</span>
        </template>
        <div class="chart-container">
          <!-- 饼图占位 -->
          <div class="chart-placeholder">
            <el-icon><PieChart /></el-icon>
            <p>销售构成图表</p>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { TrendCharts, PieChart } from '@element-plus/icons-vue'

// 趋势周期（v-model 通过 model-value + update:model-value 实现）
const emit = defineEmits<{ 'update:period': [v: string] }>()
const props = defineProps<{ period: string }>()

const trendPeriod = props.period
const updatePeriod = (v: string) => emit('update:period', v)
</script>

<style scoped>
.chart-row {
  margin-bottom: 20px;
}

.chart-card {
  height: 100%;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-container {
  height: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chart-placeholder {
  text-align: center;
  color: #999;
}

.chart-placeholder .el-icon {
  font-size: 48px;
  margin-bottom: 10px;
}
</style>
