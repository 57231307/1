<!--
  MaterialShortageSeverityCard.vue - 缺料级别进度卡片（Critical / Severe / Warning）
  拆分自 material-shortage/index.vue（P14 批 2 I-3 第 5 批）
  计数与占比都取自后端 ShortageSummary，级别取值见 @/constants/shortage
-->
<template>
  <el-row :gutter="20" class="severity-row">
    <el-col v-for="level in SHORTAGE_COUNTED_LEVELS" :key="level" :xs="24" :sm="12" :lg="8">
      <el-card shadow="hover" :class="['severity-card', levelMeta(level).className]">
        <div class="severity-content">
          <div class="severity-label">{{ getLevelText(level) }}</div>
          <div class="severity-value">{{ levelCount(level) }}</div>
        </div>
        <el-progress
          :percentage="levelPercentage(level)"
          :color="levelMeta(level).color"
          :stroke-width="8"
          :show-text="false"
        />
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import {
  SHORTAGE_COUNTED_LEVELS,
  SHORTAGE_LEVEL,
  type ShortageLevelValue,
} from '@/constants/shortage';
import { getLevelText } from '../composables/msFmts';
import type { MaterialShortageSummary } from '@/api/material-shortage';

/**
 * 缺料级别分布卡片
 */
const props = defineProps<{
  // 汇总数据
  summary: MaterialShortageSummary;
}>();

/** 级别 → 展示样式（配色与类名，与业务取值无关） */
const levelMeta = (level: ShortageLevelValue): { color: string; className: string } => {
  switch (level) {
    case SHORTAGE_LEVEL.critical:
      return { color: '#f56c6c', className: 'critical' };
    case SHORTAGE_LEVEL.severe:
      return { color: '#e6a23c', className: 'high' };
    case SHORTAGE_LEVEL.warning:
      return { color: '#409eff', className: 'medium' };
    case SHORTAGE_LEVEL.normal:
      return { color: '#909399', className: 'low' };
  }
};

/** 后端汇总只按 Critical / Severe / Warning 三档计数，Normal 表示无缺口 */
const levelCount = (level: ShortageLevelValue): number => {
  switch (level) {
    case SHORTAGE_LEVEL.critical:
      return props.summary.critical_count;
    case SHORTAGE_LEVEL.severe:
      return props.summary.severe_count;
    case SHORTAGE_LEVEL.warning:
      return props.summary.warning_count;
    case SHORTAGE_LEVEL.normal:
      return 0;
  }
};

/** 占比分母为缺料物料数；无缺料时进度为 0（除零会算出 NaN 进度条） */
const levelPercentage = (level: ShortageLevelValue): number => {
  const total = props.summary.shortage_count;
  if (!total) return 0;
  return Math.round((levelCount(level) / total) * 100);
};
</script>

<style scoped>
.severity-row {
  margin-bottom: 20px;
}
.severity-content {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 8px;
}
.severity-label {
  font-size: 13px;
  color: #606266;
}
.severity-value {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
}
.severity-card.critical .severity-value {
  color: #f56c6c;
}
.severity-card.high .severity-value {
  color: #e6a23c;
}
.severity-card.medium .severity-value {
  color: #409eff;
}
</style>
