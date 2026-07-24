<!--
  切换事件列表
  - 显示最近的切换事件流水
  - 包含时间、功能、事件类型、延迟
-->

<template>
  <el-table
    v-loading="loading"
    :data="events"
    stripe
    border
    :empty-text="$t('adminFailover.emptyEvents')"
    aria-label="故障切换事件列表"
  >
    <el-table-column prop="created_at" :label="$t('adminFailover.colTime')" width="180">
      <template #default="{ row }">
        {{ formatTime(row.created_at) }}
      </template>
    </el-table-column>
    <el-table-column prop="function_name" :label="$t('adminFailover.colFunction')" width="120">
      <template #default="{ row }">
        <el-tag size="small">{{ functionLabel(row.function_name) }}</el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="event_type" :label="$t('adminFailover.colEventType')" width="160">
      <template #default="{ row }">
        <el-tag :type="eventTagType(row.event_type)" size="small">
          {{ eventLabel(row.event_type) }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column :label="$t('adminFailover.colStateChange')" width="180">
      <template #default="{ row }">
        <span v-if="row.from_state">{{ row.from_state }} → {{ row.to_state }}</span>
        <span v-else>-</span>
      </template>
    </el-table-column>
    <el-table-column prop="reason" :label="$t('adminFailover.colReason')" min-width="200">
      <template #default="{ row }">
        <span class="reason">{{ row.reason || '-' }}</span>
      </template>
    </el-table-column>
    <el-table-column prop="latency_ms" :label="$t('adminFailover.colLatency')" width="100" align="right">
      <template #default="{ row }">
        <span v-if="row.latency_ms !== null && row.latency_ms !== undefined">
          {{ row.latency_ms }}
        </span>
        <span v-else>-</span>
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { FailoverEventDto } from '@/api/failover'

defineProps<{ events: FailoverEventDto[]; loading?: boolean }>()

const { t } = useI18n({ useScope: 'global' })

const FUNCTION_LABEL_KEYS: Record<string, string> = {
  database: 'adminFailover.funcDatabase',
  cache: 'adminFailover.funcCache',
}

const EVENT_LABEL_KEYS: Record<string, string> = {
  switch_to_backup: 'adminFailover.eventSwitchToBackup',
  switch_back: 'adminFailover.eventSwitchBack',
  primary_recovered: 'adminFailover.eventPrimaryRecovered',
  both_failed: 'adminFailover.eventBothFailed',
  circuit_open: 'adminFailover.eventCircuitOpen',
  circuit_close: 'adminFailover.eventCircuitClose',
  circuit_half_open: 'adminFailover.eventCircuitHalfOpen',
}

const EVENT_TAG_TYPES: Record<string, 'success' | 'warning' | 'danger' | 'info'> = {
  switch_to_backup: 'warning',
  switch_back: 'success',
  primary_recovered: 'success',
  both_failed: 'danger',
  circuit_open: 'danger',
  circuit_close: 'success',
  circuit_half_open: 'info',
}

function functionLabel(name: string): string {
  return FUNCTION_LABEL_KEYS[name] ? t(FUNCTION_LABEL_KEYS[name]) : name
}

function eventLabel(type: string): string {
  return EVENT_LABEL_KEYS[type] ? t(EVENT_LABEL_KEYS[type]) : type
}

function eventTagType(type: string): 'success' | 'warning' | 'danger' | 'info' {
  return EVENT_TAG_TYPES[type] || 'info'
}

function formatTime(time: string): string {
  try {
    return new Date(time).toLocaleString('zh-CN')
  } catch {
    return time
  }
}
</script>

<style scoped>
.reason {
  color: #606266;
  font-size: 13px;
}
</style>
