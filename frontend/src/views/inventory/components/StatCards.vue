<!--
  StatCards.vue - 库存统计卡片组
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的 4 统计卡片
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon total-icon">
            <el-icon><Box /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">库存总量</div>
            <div class="stat-value">{{ formatNumber(stats.totalQuantity) }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card warning">
        <div class="stat-content">
          <div class="stat-icon alert-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">库存预警</div>
            <div class="stat-value">{{ stats.alertCount }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon warehouse-icon">
            <el-icon><OfficeBuilding /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">仓库数量</div>
            <div class="stat-value">{{ stats.warehouseCount }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card danger">
        <div class="stat-content">
          <div class="stat-icon low-icon">
            <el-icon><WarningFilled /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">低于最小库存</div>
            <div class="stat-value">{{ stats.lowStockCount }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { Box, Warning, OfficeBuilding, WarningFilled } from '@element-plus/icons-vue'
import { formatNumber } from '../composables/invFmts'

defineProps<{
  stats: {
    totalQuantity: number
    alertCount: number
    warehouseCount: number
    lowStockCount: number
  }
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

.stat-card.warning {
  background: linear-gradient(135deg, #f5576c 0%, #ff6f6f 100%);
  color: white;
}

.stat-card.danger {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
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
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
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
