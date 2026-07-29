<!--
  BpmApprovalStatistics.vue - BPM 审批统计卡片（4 张）
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col v-for="card in statCards" :key="card.key" :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon" :class="card.cls">
            <el-icon><component :is="card.icon" /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">{{ card.label }}</div>
            <div class="stat-value">{{ card.value }}{{ card.key === 'avgTime' ? 'h' : '' }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Clock, CircleCheck, Warning, Timer } from '@element-plus/icons-vue';

const { t } = useI18n({ useScope: 'global' });

// 统计字段类型
interface BpmApprovalStats {
  pending: number;
  completed: number;
  urgent: number;
  avgTime: number;
}

/**
 * 审批中心统计卡片组件
 */
const props = defineProps<{
  // 统计数据
  stats: BpmApprovalStats;
}>();

// 统计卡片配置（响应式求值，随语言切换更新）
const statCards = computed(() => [
  {
    key: 'pending',
    label: t('bpm.approval.stat.pending'),
    value: props.stats.pending,
    icon: Clock,
    cls: 'pending-icon',
  },
  {
    key: 'completed',
    label: t('bpm.approval.stat.completed'),
    value: props.stats.completed,
    icon: CircleCheck,
    cls: 'completed-icon',
  },
  {
    key: 'urgent',
    label: t('bpm.approval.stat.urgent'),
    value: props.stats.urgent,
    icon: Warning,
    cls: 'urgent-icon',
  },
  {
    key: 'avgTime',
    label: t('bpm.approval.stat.avgTime'),
    value: props.stats.avgTime,
    icon: Timer,
    cls: 'avg-icon',
  },
]);
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
