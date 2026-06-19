<!--
  DbStat.vue - Dashboard 8 统计卡片（2 行 × 4 卡：面料/库存/订单/客户 + 今日/待处理/预警/销售）
  拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon fabric-icon">
            <el-icon><Folder /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">面料总数</div>
            <div class="stat-value">{{ stats.fabricCount || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon inventory-icon">
            <el-icon><Box /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">库存总量</div>
            <div class="stat-value">{{ formatNumber(stats.inventoryTotal) }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon order-icon">
            <el-icon><Document /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">本月订单</div>
            <div class="stat-value">{{ stats.monthOrders || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon customer-icon">
            <el-icon><User /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">客户总数</div>
            <div class="stat-value">{{ stats.customerCount || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>

  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card highlight">
        <div class="stat-content">
          <div class="stat-icon today-icon">
            <el-icon><Calendar /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">今日订单</div>
            <div class="stat-value">{{ stats.todayOrders || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card warning">
        <div class="stat-content">
          <div class="stat-icon pending-icon">
            <el-icon><Clock /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">待处理订单</div>
            <div class="stat-value">{{ stats.pendingOrders || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card danger">
        <div class="stat-content">
          <div class="stat-icon alert-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">库存预警</div>
            <div class="stat-value">{{ stats.lowStockProducts || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon activity-icon">
            <el-icon><TrendCharts /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">本月销售额</div>
            <div class="stat-value">{{ formatCurrency(stats.monthSales) }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { Folder, Box, Document, User, Calendar, Clock, Warning, TrendCharts } from '@element-plus/icons-vue'
import type { DashboardOverview } from '@/api/dashboard'
import { formatNumber, formatCurrency } from '../composables/dbFmts'

defineProps<{ stats: DashboardOverview }>()
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

.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.stat-card.highlight .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.highlight .stat-label,
.stat-card.highlight .stat-value {
  color: white;
}

.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

.stat-card.warning .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.warning .stat-label,
.stat-card.warning .stat-value {
  color: white;
}

.stat-card.danger {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
}

.stat-card.danger .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.danger .stat-label,
.stat-card.danger .stat-value {
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

.stat-icon.fabric-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.inventory-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: white;
}

.stat-icon.order-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
  color: white;
}

.stat-icon.customer-icon {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
  color: white;
}

.stat-icon.today-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.pending-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.alert-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.activity-icon {
  background: linear-gradient(135deg, #30cfd0 0%, #330867 100%);
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
