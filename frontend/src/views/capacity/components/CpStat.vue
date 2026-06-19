<!--
  CpStat.vue - 产能 4 统计卡片（工作中心总数/正常/繁忙/超负荷）
  拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon total-icon">
            <el-icon><OfficeBuilding /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">工作中心总数</div>
            <div class="stat-value">{{ summary.total_work_centers || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card normal">
        <div class="stat-content">
          <div class="stat-icon normal-icon">
            <el-icon><CircleCheck /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">正常运行</div>
            <div class="stat-value">{{ summary.normal_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card busy">
        <div class="stat-content">
          <div class="stat-icon busy-icon">
            <el-icon><Loading /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">繁忙状态</div>
            <div class="stat-value">{{ summary.busy_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card overload">
        <div class="stat-content">
          <div class="stat-icon overload-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">超负荷</div>
            <div class="stat-value">{{ summary.overload_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { OfficeBuilding, CircleCheck, Loading, Warning } from '@element-plus/icons-vue'
import type { CapacitySummary } from '@/api/capacity'

defineProps<{ summary: CapacitySummary }>()
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

.stat-card.normal .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.normal .stat-label,
.stat-card.normal .stat-value {
  color: white;
}

.stat-card.busy .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.busy .stat-label,
.stat-card.busy .stat-value {
  color: white;
}

.stat-card.overload .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.overload .stat-label,
.stat-card.overload .stat-value {
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

.stat-icon.normal-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
  color: white;
}

.stat-icon.busy-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.overload-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
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
</style>
