<!--
  MsSevCard.vue - 物料短缺 4 个严重程度进度卡片
  拆分自 material-shortage/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="severity-row">
    <el-col v-for="level in SEVERITY_LEVELS" :key="level.value" :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" :class="['severity-card', level.class]">
        <div class="severity-content">
          <div class="severity-label">{{ level.label }}</div>
          <div class="severity-value">{{ getCount(level.value) }}</div>
        </div>
        <el-progress
          :percentage="getPercentage(level.value)"
          :color="level.color"
          :stroke-width="8"
          :show-text="false"
        />
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { SEVERITY_LEVELS } from '../composables/msFmts'
import type { MaterialShortageSummary } from '@/api/material-shortage'

/**
 * 严重程度进度卡片
 */
const props = defineProps<{
  // 汇总数据
  summary: MaterialShortageSummary
}>()

/**
 * 获取某严重程度的数量
 */
const getCount = (severity: string) => {
  const map: Record<string, number> = {
    critical: props.summary.critical_count || 0,
    high: props.summary.high_count || 0,
    medium: props.summary.medium_count || 0,
    low: props.summary.low_count || 0,
  }
  return map[severity] || 0
}

/**
 * 获取某严重程度的占比
 */
const getPercentage = (severity: string) => {
  const count = getCount(severity)
  const total = props.summary.total_shortage_count || 0
  if (total === 0) return 0
  return Math.round((count / total) * 100)
}
</script>

<style scoped>
.severity-row {
  margin-bottom: 20px;
}
.severity-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}
.severity-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.severity-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.severity-label {
  font-size: 14px;
  font-weight: 500;
  color: #606266;
}
.severity-value {
  font-size: 24px;
  font-weight: 700;
}
.severity-card.critical {
  border-left: 4px solid #f56c6c;
}
.severity-card.high {
  border-left: 4px solid #e6a23c;
}
.severity-card.medium {
  border-left: 4px solid #409eff;
}
.severity-card.low {
  border-left: 4px solid #909399;
}
</style>
