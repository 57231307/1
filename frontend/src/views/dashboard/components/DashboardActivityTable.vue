<!--
  DashboardActivityTable.vue - Dashboard 最新活动表格
  拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>{{ t('dashboard.activityTable.title') }}</span>
        <el-button type="primary" link @click="emit('refresh')">
          <el-icon><Refresh /></el-icon>
          {{ t('dashboard.activityTable.refresh') }}
        </el-button>
      </div>
    </template>
    <el-table
      :data="data"
      stripe
      style="width: 100%"
      :aria-label="t('dashboard.activityTable.ariaLabel')"
    >
      <el-table-column prop="time" :label="t('dashboard.activityTable.colTime')" width="180">
        <template #default="{ row }">
          <el-icon><Clock /></el-icon>
          {{ row.time }}
        </template>
      </el-table-column>
      <el-table-column prop="type" :label="t('dashboard.activityTable.colType')" width="120">
        <template #default="{ row }">
          <el-tag :type="getActivityTypeColor(row.type)">{{ row.type }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="content" :label="t('dashboard.activityTable.colContent')" />
      <el-table-column prop="user" :label="t('dashboard.activityTable.colUser')" width="120" />
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { Clock, Refresh } from '@element-plus/icons-vue';
import { useI18n } from 'vue-i18n';
import type { Activity } from '@/api/dashboard';
import { getActivityTypeColor } from '../composables/dbFmts';

const { t } = useI18n({ useScope: 'global' });

defineProps<{ data: Activity[] }>();
const emit = defineEmits<{ refresh: [] }>();
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
