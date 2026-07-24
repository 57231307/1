<!--
  主备状态卡片
  - 显示单个功能（database / cache）的主备状态
  - 包含主库 / 备库 URL（脱敏）、熔断器状态、调用统计
  - 提供手动切换按钮
-->

<template>
  <el-card class="status-card" :class="cardClass">
    <template #header>
      <div class="card-header">
        <div class="title-area">
          <el-icon class="status-icon" :class="iconClass">
            <CircleCheck v-if="isPrimary" />
            <Warning v-else-if="isBackup" />
            <CircleClose v-else />
          </el-icon>
          <span class="title">{{ functionLabel }}</span>
        </div>
        <el-tag :type="stateTagType" size="large">{{ stateLabel }}</el-tag>
      </div>
    </template>

    <el-descriptions :column="1" border>
      <el-descriptions-item :label="$t('adminFailover.descPrimaryCall')">
        <span class="url">{{ maskedPrimaryUrl || $t('adminFailover.notConfigured') }}</span>
      </el-descriptions-item>
      <el-descriptions-item :label="$t('adminFailover.descBackup')">
        <span class="url">{{ backupTypeLabel }} {{ maskedBackupUrl }}</span>
      </el-descriptions-item>
      <el-descriptions-item :label="$t('adminFailover.descCircuit')">
        <el-tag :type="circuitTagType" size="small">{{ circuitLabel }}</el-tag>
        <span class="metric">{{ $t('adminFailover.failCount', { n: status.consecutive_failures }) }}</span>
      </el-descriptions-item>
      <el-descriptions-item :label="$t('adminFailover.descCallStats')">
        <span class="metric">{{ $t('adminFailover.statPrimary', { n: status.total_primary_calls.toLocaleString() }) }}</span>
        <span class="metric">{{ $t('adminFailover.statBackup', { n: status.total_backup_calls.toLocaleString() }) }}</span>
        <span class="metric">{{ $t('adminFailover.statSwitch', { n: status.total_switches.toLocaleString() }) }}</span>
      </el-descriptions-item>
      <el-descriptions-item :label="$t('adminFailover.descLastSuccess')">
        <span class="time">{{ formatTime(status.last_success_at) }}</span>
      </el-descriptions-item>
      <el-descriptions-item v-if="status.last_switch_at" :label="$t('adminFailover.descLastSwitch')">
        <span class="time">{{ formatTime(status.last_switch_at) }}</span>
      </el-descriptions-item>
    </el-descriptions>

    <div class="actions">
      <el-button
        type="warning"
        size="small"
        :icon="Switch"
        @click="$emit('switch', status.function_name)"
        :disabled="isBackup"
      >
        {{ $t('adminFailover.manualSwitch') }}
      </el-button>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  CircleCheck,
  Warning,
  CircleClose,
  Switch,
} from '@element-plus/icons-vue'
import type { FailoverStatusDto } from '@/api/failover'

const props = defineProps<{ status: FailoverStatusDto }>()
defineEmits<{ (e: 'switch', functionName: string): void }>()

const { t } = useI18n({ useScope: 'global' })

const FUNCTION_LABEL_KEYS: Record<string, string> = {
  database: 'adminFailover.funcDatabaseConn',
  cache: 'adminFailover.funcCacheService',
}

const STATE_LABEL_KEYS: Record<string, string> = {
  primary: 'adminFailover.statePrimary',
  backup: 'adminFailover.stateBackup',
  both_down: 'adminFailover.stateBothDown',
}

const STATE_TAG_TYPES: Record<string, 'success' | 'warning' | 'danger'> = {
  primary: 'success',
  backup: 'warning',
  both_down: 'danger',
}

const CIRCUIT_LABEL_KEYS: Record<string, string> = {
  closed: 'adminFailover.circuitClosed',
  open: 'adminFailover.circuitOpen',
  half_open: 'adminFailover.circuitHalfOpen',
}

const CIRCUIT_TAG_TYPES: Record<string, 'success' | 'danger' | 'warning'> = {
  closed: 'success',
  open: 'danger',
  half_open: 'warning',
}

const functionLabel = computed(() =>
  FUNCTION_LABEL_KEYS[props.status.function_name]
    ? t(FUNCTION_LABEL_KEYS[props.status.function_name])
    : props.status.function_name,
)
const stateLabel = computed(() =>
  STATE_LABEL_KEYS[props.status.current_state]
    ? t(STATE_LABEL_KEYS[props.status.current_state])
    : props.status.current_state,
)
const stateTagType = computed(() => STATE_TAG_TYPES[props.status.current_state] || 'info')
const circuitLabel = computed(() =>
  CIRCUIT_LABEL_KEYS[props.status.circuit_state]
    ? t(CIRCUIT_LABEL_KEYS[props.status.circuit_state])
    : props.status.circuit_state,
)
const circuitTagType = computed(() => CIRCUIT_TAG_TYPES[props.status.circuit_state] || 'info')

const isPrimary = computed(() => props.status.current_state === 'primary')
const isBackup = computed(() => props.status.current_state === 'backup')

const cardClass = computed(() => ({
  'is-primary': isPrimary.value,
  'is-backup': isBackup.value,
  'is-down': props.status.current_state === 'both_down',
}))

const iconClass = computed(() => ({
  'icon-primary': isPrimary.value,
  'icon-backup': isBackup.value,
  'icon-down': props.status.current_state === 'both_down',
}))

const backupTypeLabel = computed(() => {
  if (props.status.function_name === 'database') return t('adminFailover.backupPostgres')
  if (props.status.function_name === 'cache') return t('adminFailover.backupLru')
  return props.status.backup_type || t('adminFailover.stateUnknown')
})

/** 脱敏主 URL */
const maskedPrimaryUrl = computed(() => {
  const url = props.status.primary_url
  if (!url) return ''
  // 隐藏密码部分
  return url.replace(/:[^:@]+@/, ':***@')
})

/** 脱敏备 URL */
const maskedBackupUrl = computed(() => {
  if (props.status.backup_type === 'lru') return t('adminFailover.memoryCache')
  return ''
})

/** 格式化时间 */
function formatTime(time?: string): string {
  if (!time) return t('adminFailover.never')
  try {
    return new Date(time).toLocaleString('zh-CN')
  } catch {
    return time
  }
}
</script>

<style scoped>
.status-card {
  height: 100%;
}

.status-card.is-backup {
  border-color: #e6a23c;
}

.status-card.is-down {
  border-color: #f56c6c;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title-area {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-icon {
  font-size: 20px;
}

.icon-primary {
  color: #67c23a;
}

.icon-backup {
  color: #e6a23c;
}

.icon-down {
  color: #f56c6c;
}

.title {
  font-size: 16px;
  font-weight: 600;
}

.url {
  font-family: 'Courier New', monospace;
  font-size: 12px;
  color: #606266;
}

.metric {
  margin-left: 12px;
  font-family: 'Courier New', monospace;
  font-size: 13px;
}

.time {
  color: #909399;
  font-size: 13px;
}

.actions {
  margin-top: 16px;
  text-align: right;
}
</style>
