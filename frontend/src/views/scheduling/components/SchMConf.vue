<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchMConf.vue - 排产冲突侧栏
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
-->
<template>
  <el-card shadow="hover" class="conflict-card">
    <template #header>
      <div class="card-header">
        <span>冲突检测</span>
        <el-tag :type="conflictList.length > 0 ? 'danger' : 'success'" size="small">
          {{ conflictList.length }} 个冲突
        </el-tag>
      </div>
    </template>
    <div class="conflict-actions">
      <el-button type="warning" size="small" :loading="conflictLoading" @click="emit('detect')">
        <el-icon><Search /></el-icon>
        检测冲突
      </el-button>
    </div>
    <div v-loading="conflictLoading" class="conflict-list">
      <div v-if="conflictList.length === 0" class="empty-state">
        <el-icon><CircleCheck /></el-icon>
        <p>暂无排程冲突</p>
      </div>
      <div v-for="item in conflictList" :key="item.id" class="conflict-item">
        <div class="conflict-header">
          <span class="conflict-wc">{{ item.work_center_name }}</span>
          <el-tag :type="item.severity === 'error' ? 'danger' : 'warning'" size="small">
            {{ item.severity === 'error' ? '严重' : '警告' }}
          </el-tag>
        </div>
        <div class="conflict-orders">
          <el-tag size="small" type="info">{{ item.order_no_1 }}</el-tag>
          <el-icon style="margin: 0 4px"><Switch /></el-icon>
          <el-tag size="small" type="info">{{ item.order_no_2 }}</el-tag>
        </div>
        <div class="conflict-time">
          <el-icon><Time /></el-icon>
          <span>{{ formatTime(item.overlap_start) }} ~ {{ formatTime(item.overlap_end) }}</span>
        </div>
        <div class="conflict-suggestion">
          <el-icon><ChatDotRound /></el-icon>
          <span>{{ item.suggestion }}</span>
        </div>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { ConflictItem } from '@/api/scheduling'

// 排产冲突侧栏属性
defineProps<{
  // 冲突列表
  conflictList: ConflictItem[]
  // 冲突加载状态
  conflictLoading: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 检测冲突
  (e: 'detect'): void
}>()

/** 格式化时间 */
const formatTime = (t: string) => {
  if (!t) return '-'
  return t.replace('T', ' ').slice(0, 16)
}
</script>

<style scoped>
.conflict-card {
  margin-bottom: 20px;
  border-radius: 12px;
}

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

.conflict-actions {
  margin-bottom: 16px;
}

.conflict-list {
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

.conflict-item {
  padding: 12px;
  border-left: 3px solid #f56c6c;
  background: #fef0f0;
  border-radius: 4px;
  margin-bottom: 12px;
}

.conflict-item:last-child {
  margin-bottom: 0;
}

.conflict-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.conflict-wc {
  font-weight: 600;
  color: #303133;
  font-size: 14px;
}

.conflict-orders {
  display: flex;
  align-items: center;
  margin-bottom: 6px;
}

.conflict-time,
.conflict-suggestion {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #606266;
  margin-bottom: 4px;
}

.conflict-time .el-icon,
.conflict-suggestion .el-icon {
  font-size: 14px;
}
</style>
