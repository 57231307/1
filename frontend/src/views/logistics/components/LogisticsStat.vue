<!--
  LogisticsStat.vue - 物流管理统计卡片（4 张）
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  总运单数取列表接口的全库 total，三张状态卡按当前页数据计数
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('logistics.stat.label.total') }}</div>
          <div class="stat-value">{{ stats.total || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('logistics.stat.label.inTransit') }}</div>
          <div class="stat-value text-primary">{{ stats.inTransit || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('logistics.stat.label.delivered') }}</div>
          <div class="stat-value text-warning">{{ stats.delivered || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">{{ t('logistics.stat.label.signed') }}</div>
          <div class="stat-value text-success">{{ stats.signed || 0 }}</div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

// 统计字段类型（与运单状态机的三个状态一一对应）
interface LgsStats {
  total: number;
  inTransit: number;
  delivered: number;
  signed: number;
}

/**
 * 物流统计卡片组件
 */
defineProps<{
  // 统计数据
  stats: LgsStats;
}>();
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
.text-primary {
  color: #409eff;
}
.text-success {
  color: #67c23a;
}
</style>
