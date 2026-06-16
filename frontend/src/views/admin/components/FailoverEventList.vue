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
    empty-text="暂无切换事件"
  >
    <el-table-column prop="created_at" label="时间" width="180">
      <template #default="{ row }">
        {{ formatTime(row.created_at) }}
      </template>
    </el-table-column>
    <el-table-column prop="function_name" label="功能" width="120">
      <template #default="{ row }">
        <el-tag size="small">{{ functionLabel(row.function_name) }}</el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="event_type" label="事件类型" width="160">
      <template #default="{ row }">
        <el-tag :type="eventTagType(row.event_type)" size="small">
          {{ eventLabel(row.event_type) }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column label="状态变更" width="180">
      <template #default="{ row }">
        <span v-if="row.from_state">{{ row.from_state }} → {{ row.to_state }}</span>
        <span v-else>-</span>
      </template>
    </el-table-column>
    <el-table-column prop="reason" label="原因" min-width="200">
      <template #default="{ row }">
        <span class="reason">{{ row.reason || '-' }}</span>
      </template>
    </el-table-column>
    <el-table-column prop="latency_ms" label="延迟(ms)" width="100" align="right">
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
import type { FailoverEventDto } from '@/api/failover'

defineProps<{ events: FailoverEventDto[]; loading?: boolean }>()

const FUNCTION_LABELS: Record<string, string> = {
  database: '数据库',
  cache: '缓存',
}

const EVENT_LABELS: Record<string, string> = {
  switch_to_backup: '切换至备用',
  switch_back: '回切至主',
  primary_recovered: '主恢复',
  both_failed: '主备均失败',
  circuit_open: '熔断器打开',
  circuit_close: '熔断器关闭',
  circuit_half_open: '熔断器半开',
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
  return FUNCTION_LABELS[name] || name
}

function eventLabel(type: string): string {
  return EVENT_LABELS[type] || type
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
