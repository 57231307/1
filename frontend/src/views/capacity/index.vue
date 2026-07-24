<!--
  capacity/index.vue - 产能分析（拆分重构版）
  任务编号: P14 批 2 I-3 第 6 批
  拆分：562 行 → ~150 行 + 4 子组件 + 1 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="capacity-page">
    <div class="page-header">
      <h2>产能分析</h2>
      <div class="header-actions">
        <el-date-picker
          v-model="cp.dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @change="cp.handleDateChange"
        />
        <el-select
          v-model="cp.selectedWorkCenter"
          placeholder="选择工作中心"
          clearable
          style="width: 200px; margin-left: 12px"
          @change="cp.handleWorkCenterChange"
        >
          <el-option label="全部" :value="undefined" />
          <el-option
            v-for="wc in cp.workCenters"
            :key="wc.id"
            :label="wc.name"
            :value="wc.id"
          />
        </el-select>
      </div>
    </div>

    <CapacityStat :summary="cp.summary" />

    <CapacityTrend :data="cp.trendData" :days="cp.trendDays" @update:days="cp.handleTrendDaysChange" />

    <el-row :gutter="20" class="table-row">
      <el-col :xs="24" :lg="16">
        <CapacityTable
          :data="cp.workCenters"
          :table-loading="cp.tableLoading"
          :total="cp.total"
          :page="cp.currentPage"
          :page-size="cp.pageSize"
          @refresh="cp.fetchWorkCenters"
          @update:page="(v: number) => (cp.currentPage = v)"
          @update:size="(v: number) => (cp.pageSize = v)"
        />
      </el-col>
      <el-col :xs="24" :lg="8">
        <CapacityBottleneck :data="cp.bottlenecks" :loading="cp.bottleneckLoading" />
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useCp } from './composables/useCp'
import CapacityStat from './components/CapacityStat.vue'
import CapacityTrend from './components/CapacityTrend.vue'
import CapacityTable from './components/CapacityTable.vue'
import CapacityBottleneck from './components/CapacityBottleneck.vue'

// 业务状态
const cp = useCp()

onMounted(() => {
  cp.initOnMount()
})
</script>

<style scoped>
.capacity-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.page-header h2 {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.header-actions {
  display: flex;
  align-items: center;
}

.table-row {
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
