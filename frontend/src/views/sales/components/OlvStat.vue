<!--
  OlvStat.vue - 销售订单列表 4 统计卡片
  拆分自 sales/views/OrderListView.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="stats-cards">
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stats-item">
          <div class="stats-label">订单总数</div>
          <div class="stats-value">{{ stats.totalCount }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stats-item">
          <div class="stats-label">待审批</div>
          <div class="stats-value warning">{{ stats.pendingCount }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stats-item">
          <div class="stats-label">已审批</div>
          <div class="stats-value success">{{ stats.approvedCount }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="6">
      <el-card shadow="hover">
        <div class="stats-item">
          <div class="stats-label">订单总额</div>
          <div class="stats-value highlight">¥{{ stats.totalAmount.toLocaleString() }}</div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
// 销售订单统计类型
interface OlvStats {
  totalCount: number
  pendingCount: number
  approvedCount: number
  totalAmount: number
}

/**
 * 销售订单列表 4 统计卡片组件
 */
defineProps<{
  // 统计数据
  stats: OlvStats
}>()
</script>

<style scoped>
.stats-cards {
  margin-bottom: 20px;
}
.stats-item {
  text-align: center;
  padding: 10px 0;
}
.stats-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}
.stats-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
}
.stats-value.warning {
  color: #e6a23c;
}
.stats-value.success {
  color: #67c23a;
}
.stats-value.highlight {
  color: #f56c6c;
}
</style>
