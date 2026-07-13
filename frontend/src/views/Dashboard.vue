<!--
  Dashboard.vue - 仪表盘（拆分重构版）
  任务编号: P14 批 2 I-3 第 6 批
  拆分：549 行 → ~100 行 + 4 子组件 + 1 composable + 1 工具
  行为完全保持一致（仅结构重构）
  注：路由直接引用 views/Dashboard.vue，组件放在子目录 dashboard/
-->
<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1 class="dashboard-title">{{ $t('dashboard.headerTitle') }}</h1>
      <el-date-picker
        v-model="db.dateRange"
        type="daterange"
        :range-separator="$t('dashboard.dateRange.to')"
        :start-placeholder="$t('dashboard.dateRange.startPlaceholder')"
        :end-placeholder="$t('dashboard.dateRange.endPlaceholder')"
        @change="db.handleDateChange"
      />
    </div>

    <DbStat :stats="db.stats" />

    <el-row :gutter="20" class="charts-row">
      <el-col :xs="24" :lg="16">
        <DbTrend
          :data="db.trendData"
          :days="db.trendDays"
          @update:days="(v: number) => (db.trendDays = v)"
        />
      </el-col>
      <el-col :xs="24" :lg="8">
        <DbPie :data="db.categoryDistribution" />
      </el-col>
    </el-row>

    <el-row :gutter="20" class="activities-row">
      <el-col :span="24">
        <DbActTbl :data="db.stats.recentActivities || []" @refresh="db.refreshActivities" />
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useDb } from './dashboard/composables/useDb'
import DbStat from './dashboard/components/DbStat.vue'
import DbTrend from './dashboard/components/DbTrend.vue'
import DbPie from './dashboard/components/DbPie.vue'
import DbActTbl from './dashboard/components/DbActTbl.vue'

// 业务状态
const db = useDb()

onMounted(async () => {
  await db.fetchDashboardData()
  await db.fetchChartData()
})
</script>

<style scoped>
.dashboard {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.dashboard-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.charts-row {
  margin-bottom: 20px;
}

.activities-row {
  margin-bottom: 20px;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
