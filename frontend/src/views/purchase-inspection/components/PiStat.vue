<!--
  PiStat.vue - 采购验货统计卡片（4 张）
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">总检验单数</div>
          <div class="stat-value">{{ stats.total || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">待检验</div>
          <div class="stat-value text-warning">{{ stats.pending || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">检验合格</div>
          <div class="stat-value text-success">{{ stats.passed || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">检验不合格</div>
          <div class="stat-value text-danger">{{ stats.failed || 0 }}</div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
/**
 * 统计字段类型
 */
interface PiStats {
  total: number
  pending: number
  passed: number
  failed: number
}

/**
 * 采购验货统计卡片
 */
defineProps<{
  // 统计数据
  stats: PiStats
}>()
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-item {
  text-align: center;
}
.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 10px;
}
.stat-value {
  font-size: 28px;
  font-weight: 600;
}
.text-warning {
  color: #e6a23c;
}
.text-success {
  color: #67c23a;
}
.text-danger {
  color: #f56c6c;
}
</style>
