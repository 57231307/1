<!--
  MaterialShortageStat.vue - 物料缺料 4 张统计卡片
  拆分自 material-shortage/index.vue（P14 批 2 I-3 第 5 批）
  字段取自后端 ShortageSummary（实时检测结果汇总）
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card total">
        <div class="stat-content">
          <div class="stat-icon total-icon">
            <el-icon><Files /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">{{ t('materialShortage.stat.materialsChecked') }}</div>
            <div class="stat-value">{{ summary.total_materials_checked }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card high">
        <div class="stat-content">
          <div class="stat-icon high-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">{{ t('materialShortage.stat.shortageCount') }}</div>
            <div class="stat-value">{{ summary.shortage_count }}</div>
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
            <div class="stat-label">{{ t('materialShortage.stat.criticalShortage') }}</div>
            <div class="stat-value">{{ summary.critical_count }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card info">
        <div class="stat-content">
          <div class="stat-icon info-icon">
            <el-icon><Tickets /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">{{ t('materialShortage.stat.affectedOrders') }}</div>
            <div class="stat-value">{{ summary.affected_orders_count }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Files, Warning, CircleClose, Tickets } from '@element-plus/icons-vue';
import type { MaterialShortageSummary } from '@/api/material-shortage';

const { t } = useI18n({ useScope: 'global' });

/**
 * 统计卡片组件
 */
defineProps<{
  // 汇总数据
  summary: MaterialShortageSummary;
}>();
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  margin-bottom: 0;
}
.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}
.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: #fff;
}
.total-icon {
  background: #409eff;
}
.critical-icon {
  background: #f56c6c;
}
.high-icon {
  background: #e6a23c;
}
.info-icon {
  background: #909399;
}
.stat-info {
  flex: 1;
}
.stat-label {
  font-size: 13px;
  color: #909399;
  margin-bottom: 6px;
}
.stat-value {
  font-size: 22px;
  font-weight: 600;
  color: #303133;
}
</style>
