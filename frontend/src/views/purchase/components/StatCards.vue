<script setup lang="ts">
/**
 * StatCards - 采购管理统计卡片（4 个指标）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 统计卡片）
 */
import { Document, Money, Clock, OfficeBuilding } from '@element-plus/icons-vue'

interface Stats {
  monthOrders: number
  monthAmount: number
  pendingReceipt: number
  supplierCount: number
}

interface Props {
  stats: Stats
  formatCurrency: (amount: number) => string
}

defineProps<Props>()
</script>

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
