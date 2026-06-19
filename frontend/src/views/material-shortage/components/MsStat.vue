<!--
  MsStat.vue - 物料短缺 4 张统计卡片
  拆分自 material-shortage/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card total">
        <div class="stat-content">
          <div class="stat-icon total-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">缺料总数</div>
            <div class="stat-value">{{ summary.total_shortage_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card critical">
        <div class="stat-content">
          <div class="stat-icon critical-icon">
            <el-icon><CircleClose /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">严重缺料</div>
            <div class="stat-value">{{ summary.critical_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card high">
        <div class="stat-content">
          <div class="stat-icon high-icon">
            <el-icon><WarningFilled /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">高度缺料</div>
            <div class="stat-value">{{ summary.high_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card info">
        <div class="stat-content">
          <div class="stat-icon info-icon">
            <el-icon><Clock /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">最后检查时间</div>
            <div class="stat-value time-value">{{ summary.last_check_time || '-' }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { Warning, CircleClose, WarningFilled, Clock } from '@element-plus/icons-vue'
import type { MaterialShortageSummary } from '@/api/material-shortage'

/**
 * 统计卡片组件
 */
defineProps<{
  // 汇总数据
  summary: MaterialShortageSummary
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
.stat-card.total .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.total .stat-label,
.stat-card.total .stat-value {
  color: white;
}
.stat-card.critical .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.critical .stat-label,
.stat-card.critical .stat-value {
  color: white;
}
.stat-card.high .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.high .stat-label,
.stat-card.high .stat-value {
  color: white;
}
.stat-card.info .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.info .stat-label,
.stat-card.info .stat-value {
  color: white;
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
}
.stat-icon.total-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}
.stat-icon.critical-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
  color: white;
}
.stat-icon.high-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}
.stat-icon.info-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: white;
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
.time-value {
  font-size: 14px;
  font-weight: 500;
}
</style>
