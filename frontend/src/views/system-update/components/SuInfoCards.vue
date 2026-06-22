<!--
  SuInfoCards.vue - 系统更新顶部信息卡
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-row :gutter="20" class="info-cards">
    <el-col :span="8">
      <el-card shadow="hover">
        <template #header>
          <span>当前版本</span>
        </template>
        <div class="info-content">
          <div class="version">{{ currentVersion?.version || '-' }}</div>
          <div class="date">构建日期: {{ currentVersion?.build_date || '-' }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="8">
      <el-card shadow="hover">
        <template #header>
          <span>最新版本</span>
        </template>
        <div class="info-content">
          <div class="version">{{ latestVersion?.version || '-' }}</div>
          <div class="date">发布日期: {{ latestVersion?.release_date || '-' }}</div>
        </div>
      </el-card>
    </el-col>
    <el-col :span="8">
      <el-card shadow="hover">
        <template #header>
          <span>更新状态</span>
        </template>
        <div class="info-content">
          <el-tag :type="hasUpdate ? 'warning' : 'success'" size="large">
            {{ hasUpdate ? '有可用更新' : '已是最新版本' }}
          </el-tag>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import type { SystemVersion } from '@/api/system-update'

/**
 * 系统更新顶部信息卡组件
 * 显示当前版本、最新版本、更新状态
 */
const props = defineProps<{
  // 当前版本
  currentVersion: { version: string; build_date: string } | null
  // 最新版本
  latestVersion: SystemVersion | null
  // 是否有可用更新
  hasUpdate: boolean
}>()

void props
</script>

<style scoped>
.info-cards {
  margin-bottom: 20px;
}
.info-content {
  text-align: center;
  padding: 10px 0;
}
.version {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 8px;
}
.date {
  font-size: 14px;
  color: #909399;
}
</style>
