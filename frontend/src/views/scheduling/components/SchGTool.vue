<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchGTool.vue - 排产甘特图顶部工具栏 + 统计卡片
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
-->
<template>
  <div class="page-header">
    <div class="header-left">
      <el-button link @click="emit('back')">
        <el-icon><ArrowLeft /></el-icon>
        返回排程管理
      </el-button>
      <h2>生产排程甘特图</h2>
    </div>
    <div class="header-actions">
      <el-date-picker
        :model-value="dateRange"
        type="daterange"
        range-separator="至"
        start-placeholder="开始日期"
        end-placeholder="结束日期"
        style="width: 240px"
        @update:model-value="onDateChange"
      />
      <el-button type="primary" :loading="scheduling" @click="emit('auto-schedule')">
        <el-icon><Cpu /></el-icon>
        自动排程
      </el-button>
      <el-button @click="emit('refresh')">
        <el-icon><Refresh /></el-icon>
        刷新
      </el-button>
    </div>
  </div>

  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon task-icon">
            <el-icon><List /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">总任务数</div>
            <div class="stat-value">{{ ganttData.total_tasks || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card conflict">
        <div class="stat-content">
          <div class="stat-icon conflict-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">冲突数</div>
            <div class="stat-value">{{ ganttData.conflict_count || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon wc-icon">
            <el-icon><OfficeBuilding /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">工作中心</div>
            <div class="stat-value">{{ ganttData.work_centers?.length || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon range-icon">
            <el-icon><Calendar /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">排程范围</div>
            <div class="stat-value range-text">{{ dateRangeText }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
// 排产甘特图数据类型（最少必要字段）
interface GanttDataLite {
  total_tasks?: number
  conflict_count?: number
  work_centers?: Array<unknown>
}

// 排产甘特图工具栏属性
const props = defineProps<{
  // 甘特图数据
  ganttData: GanttDataLite
  // 日期范围
  dateRange: [Date, Date] | null
  // 日期范围文本
  dateRangeText: string
  // 自动排程进行中
  scheduling: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 返回
  (e: 'back'): void
  // 刷新
  (e: 'refresh'): void
  // 自动排程
  (e: 'auto-schedule'): void
  // 日期变化
  (e: 'date-change', value: [Date, Date] | null): void
}>()

/** 日期范围变化 */
const onDateChange = (v: [Date, Date] | null) => {
  emit('date-change', v)
  emit('refresh')
}
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.header-left h2 {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}

.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.stat-card.conflict .stat-icon {
  background: rgba(255, 255, 255, 0.2);
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
}

.stat-icon.task-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.conflict-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
  color: white;
}

.stat-icon.wc-icon {
  background: linear-gradient(135deg, #a18cd1 0%, #fbc2eb 100%);
  color: white;
}

.stat-icon.range-icon {
  background: linear-gradient(135deg, #89f7fe 0%, #66a6ff 100%);
  color: white;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}

.range-text {
  font-size: 14px !important;
  color: #606266 !important;
}
</style>
