<!--
  DashboardActivityTable.vue - Dashboard 最新活动表格
  拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>最新活动</span>
        <el-button type="primary" link @click="emit('refresh')">
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </template>
    <el-table :data="data" stripe style="width: 100%" aria-label="最新活动表格">
      <el-table-column prop="time" label="时间" width="180">
        <template #default="{ row }">
          <el-icon><Clock /></el-icon>
          {{ row.time }}
        </template>
      </el-table-column>
      <el-table-column prop="type" label="类型" width="120">
        <template #default="{ row }">
          <el-tag :type="getActivityTypeColor(row.type)">{{ row.type }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="content" label="内容" />
      <el-table-column prop="user" label="操作人" width="120" />
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { Clock, Refresh } from '@element-plus/icons-vue'
import type { Activity } from '@/api/dashboard'
import { getActivityTypeColor } from '../composables/dbFmts'

defineProps<{ data: Activity[] }>()
const emit = defineEmits<{ refresh: [] }>()
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
</style>
