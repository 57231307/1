<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchMTool.vue - 排产管理顶部工具栏 + 统计卡片
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
-->
<template>
  <div class="page-header">
    <h2>生产排程管理</h2>
    <div class="header-actions">
      <el-button type="primary" :loading="scheduling" @click="emit('auto-schedule')">
        <el-icon><Cpu /></el-icon>
        自动排程
      </el-button>
      <el-button @click="emit('goto-gantt')">
        <el-icon><TrendCharts /></el-icon>
        查看甘特图
      </el-button>
    </div>
  </div>

  <el-row :gutter="20" class="stats-row">
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon pending-icon">
            <el-icon><Clock /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">待排程工单</div>
            <div class="stat-value">{{ stats.pending || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon scheduled-icon">
            <el-icon><Calendar /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">已排程工单</div>
            <div class="stat-value">{{ stats.scheduled || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon running-icon">
            <el-icon><Loading /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">生产中工单</div>
            <div class="stat-value">{{ stats.running || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
    <el-col :xs="24" :sm="12" :lg="6">
      <el-card shadow="hover" class="stat-card">
        <div class="stat-content">
          <div class="stat-icon conflict-icon">
            <el-icon><Warning /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-label">冲突数量</div>
            <div class="stat-value">{{ stats.conflicts || 0 }}</div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
// 统计数据类型
interface Stats {
  pending: number
  scheduled: number
  running: number
  conflicts: number
}

// 排产管理工具栏属性
defineProps<{
  // 统计数据
  stats: Stats
  // 自动排程进行中
  scheduling: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 自动排程
  (e: 'auto-schedule'): void
  // 跳转到甘特图
  (e: 'goto-gantt'): void
}>()
</script>

<style scoped>
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

.stat-icon.pending-icon {
  background: linear-gradient(135deg, #a8edea 0%, #fed6e3 100%);
  color: white;
}

.stat-icon.scheduled-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.running-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.conflict-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
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
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
</style>
