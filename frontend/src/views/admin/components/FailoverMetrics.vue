<!--
  健康检查指标展示
  - 显示当前主备健康状态
-->

<template>
  <div class="metrics-display">
    <el-row :gutter="20">
      <el-col :span="12">
        <div class="metric-card" :class="databaseClass">
          <div class="metric-label">{{ $t('adminFailover.metricDatabase') }}</div>
          <div class="metric-value">{{ databaseLabel }}</div>
        </div>
      </el-col>
      <el-col :span="12">
        <div class="metric-card" :class="cacheClass">
          <div class="metric-label">{{ $t('adminFailover.metricCache') }}</div>
          <div class="metric-value">{{ cacheLabel }}</div>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  health: { database: string; cache: string }
}>()

const { t } = useI18n({ useScope: 'global' })

const STATE_LABEL_KEYS: Record<string, string> = {
  primary: 'adminFailover.statePrimary',
  backup: 'adminFailover.stateBackup',
  both_down: 'adminFailover.stateBothDown',
  error: 'adminFailover.stateError',
}

function stateLabel(state: string): string {
  return STATE_LABEL_KEYS[state] ? t(STATE_LABEL_KEYS[state]) : t('adminFailover.stateUnknown')
}

const databaseLabel = computed(() => stateLabel(props.health.database))
const cacheLabel = computed(() => stateLabel(props.health.cache))

const databaseClass = computed(() => stateClass(props.health.database))
const cacheClass = computed(() => stateClass(props.health.cache))

function stateClass(state: string): string {
  switch (state) {
    case 'primary':
      return 'is-primary'
    case 'backup':
      return 'is-backup'
    case 'both_down':
    case 'error':
      return 'is-down'
    default:
      return 'is-unknown'
  }
}
</script>

<style scoped>
.metrics-display {
  padding: 10px 0;
}

.metric-card {
  padding: 20px;
  border-radius: 8px;
  text-align: center;
  background: #f5f7fa;
  border: 1px solid #ebeef5;
  transition: all 0.3s;
}

.metric-card.is-primary {
  background: #f0f9eb;
  border-color: #b3e19d;
}

.metric-card.is-backup {
  background: #fdf6ec;
  border-color: #f3d19e;
}

.metric-card.is-down {
  background: #fef0f0;
  border-color: #fab6b6;
}

.metric-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}

.metric-value {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}
</style>
