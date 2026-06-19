<!--
  LgsStat.vue - 物流管理统计卡片（4 张）
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-row :gutter="20" class="stats-row">
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">总运单数</div>
          <div class="stat-value">{{ stats.total || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">待发货</div>
          <div class="stat-value text-warning">{{ stats.pending || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">运输中</div>
          <div class="stat-value text-primary">{{ stats.inTransit || 0 }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stat-item">
          <div class="stat-label">已签收</div>
          <div class="stat-value text-success">{{ stats.delivered || 0 }}</div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */

// 统计字段类型
interface LgsStats {
  total: number
  pending: number
  inTransit: number
  delivered: number
}

/**
 * 物流统计卡片组件
 */
defineProps<{
  // 统计数据
  stats: LgsStats
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
.text-primary {
  color: #409eff;
}
.text-success {
  color: #67c23a;
}
</style>
