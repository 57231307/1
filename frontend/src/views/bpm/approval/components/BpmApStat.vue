<!--
  BpmApStat.vue - BPM 审批统计卡片（4 张）
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card pending">
        <div class="stat-content">
          <div class="stat-icon pending-icon">
            <el-icon><Clock /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">待办任务</div>
            <div class="stat-value">{{ stats.pending }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card completed">
        <div class="stat-content">
          <div class="stat-icon completed-icon">
            <el-icon><CircleCheck /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">已办任务</div>
            <div class="stat-value">{{ stats.completed }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card urgent">
        <div class="stat-content">
          <div class="stat-icon urgent-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">紧急任务</div>
            <div class="stat-value">{{ stats.urgent }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card avg">
        <div class="stat-content">
          <div class="stat-icon avg-icon">
            <el-icon><Timer /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">平均处理时长</div>
            <div class="stat-value">{{ stats.avgTime }}h</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Clock, CircleCheck, Warning, Timer } from '@element-plus/icons-vue'

// 统计字段类型
interface BpmApStats {
  pending: number
  completed: number
  urgent: number
  avgTime: number
}

/**
 * 审批中心统计卡片组件
 */
defineProps<{
  // 统计数据
  stats: BpmApStats
}>()
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}
.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: white;
}
.pending-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.completed-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.urgent-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.avg-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-info {
  flex: 1;
}
.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
</style>
