<!--
  PurchaseStatsCards.vue - 采购统计卡片组件
  来源：原 purchase/index.vue 中 顶部统计卡片区
  拆分日期：2026-06-17 P1-3-Batch-2
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon order-icon">
            <el-icon><Document /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">本月采购</div>
            <div class="stat-value">{{ stats.monthOrders }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card highlight">
        <div class="stat-content">
          <div class="stat-icon amount-icon">
            <el-icon><Money /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">采购金额</div>
            <div class="stat-value">{{ formatCurrency(stats.monthAmount) }}</div>
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
            <div class="stat-label">待收货</div>
            <div class="stat-value">{{ stats.pendingReceipt }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon supplier-icon">
            <el-icon><OfficeBuilding /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">合作供应商</div>
            <div class="stat-value">{{ stats.supplierCount }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { Document, Money, Clock, OfficeBuilding } from '@element-plus/icons-vue'

interface PurchaseStats {
  monthOrders: number
  monthAmount: number
  pendingReceipt: number
  supplierCount: number
}

withDefaults(
  defineProps<{
    stats: PurchaseStats
  }>(),
  {}
)

const formatCurrency = (amount: number) => {
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 0,
  }).format(amount)
}
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
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}

.stat-card.warning {
  background: linear-gradient(135deg, #f5576c 0%, #ff6f6f 100%);
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

.stat-icon.order-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.stat-icon.amount-icon {
  background: rgba(255, 255, 255, 0.2);
}

.stat-icon.pending-icon {
  background: rgba(255, 255, 255, 0.2);
}

.stat-icon.supplier-icon {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
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
