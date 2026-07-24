<!--
  CapacityBottleneck.vue - 瓶颈识别侧栏
  拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>{{ $t('capacityModule.bottleneck.title') }}</span>
        <el-tag type="danger">{{ $t('capacityModule.bottleneck.count', { count: data.length }) }}</el-tag>
      </div>
    </template>
    <div v-loading="loading" class="bottleneck-list">
      <div v-if="data.length === 0" class="empty-state">
        <el-icon><CircleCheck /></el-icon>
        <p>{{ $t('capacityModule.bottleneck.empty') }}</p>
      </div>
      <div v-for="item in data" :key="item.id" class="bottleneck-item">
        <div class="bottleneck-header">
          <span class="bottleneck-name">{{ item.name }}</span>
          <el-tag type="danger" size="small">{{ $t('capacityModule.bottleneck.tag') }}</el-tag>
        </div>
        <div class="bottleneck-info">
          <span>{{ $t('capacityModule.bottleneck.loadRate') }}: <strong>{{ (item.load_rate * 100).toFixed(1) }}%</strong></span>
          <span>{{ $t('capacityModule.bottleneck.usedHours') }}: {{ item.used_hours }} / {{ item.capacity_hours }}</span>
        </div>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { CircleCheck } from '@element-plus/icons-vue'
import type { WorkCenter } from '@/api/capacity'

defineProps<{
  data: WorkCenter[]
  loading: boolean
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header span {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.bottleneck-list {
  min-height: 200px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
  color: #909399;
}

.empty-state .el-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.bottleneck-item {
  padding: 12px;
  border-bottom: 1px solid #ebeef5;
}

.bottleneck-item:last-child {
  border-bottom: none;
}

.bottleneck-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.bottleneck-name {
  font-weight: 600;
  color: #303133;
}

.bottleneck-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
  color: #606266;
}
</style>
