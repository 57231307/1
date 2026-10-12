<!--
  PurchaseInspectionStat.vue - 采购验货统计卡片（4 张）
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  数据为服务端聚合（GET /purchase/inspections/stats）的只读展示层：
  stats 为 null 表示未取到（失败/未返回），数值渲染「—」明确留空，不兜底 0。
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('purchaseInspection.stat.label.total') }}</div>
          <div class="stat-value">{{ fmt(stats?.total) }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('purchaseInspection.stat.label.pending') }}</div>
          <div class="stat-value text-warning">{{ fmt(stats?.pending) }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('purchaseInspection.stat.label.passed') }}</div>
          <div class="stat-value text-success">{{ fmt(stats?.passed) }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('purchaseInspection.stat.label.failed') }}</div>
          <div class="stat-value text-danger">{{ fmt(stats?.failed) }}</div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
// 统计形状单一来源 = api 层按后端 DTO 声明的类型，组件不再本地复制 interface
import type { PurchaseInspectionStats } from '@/api/purchase-inspection';

const { t } = useI18n({ useScope: 'global' });

/**
 * 采购验货统计卡片（服务端聚合四值的只读展示）
 */
defineProps<{
  // null = 未取到（首次未返回或取数失败），四卡显示「—」；数字 0 与未知必须可区分
  stats: PurchaseInspectionStats | null;
}>();

/** 未知态与 0 严格区分：仅有限数字直显，其余一律占位符，不 ?? 0 */
const fmt = (v: number | undefined): string =>
  typeof v === 'number' && Number.isFinite(v) ? String(v) : '—';
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-item {
  text-align: center;
}
.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 10px;
}
.stat-value {
  font-size: 28px;
  font-weight: 600;
}
.text-warning {
  color: #e6a23c;
}
.text-success {
  color: #67c23a;
}
.text-danger {
  color: #f56c6c;
}
</style>
