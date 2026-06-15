<!--
  库存台账 - el-table-v2 虚拟滚动 POC 演示入口

  路径: /inventory-poc
  任务: B5 P2-1 验证 el-table-v2 替代原生 el-table 的可行性
-->
<template>
  <div class="inventory-poc-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">
          库存台账 - el-table-v2 虚拟列表 POC
        </h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>研发验证</el-breadcrumb-item>
          <el-breadcrumb-item>B5 P2-1 虚拟列表 POC</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-tag type="info" size="small">Element Plus 2.14.1</el-tag>
        <el-button @click="goBack">
          <el-icon><Back /></el-icon>
          返回
        </el-button>
      </div>
    </div>

    <el-alert
      class="poc-alert"
      type="info"
      show-icon
      :closable="false"
      title="POC 目标"
      description="验证 Element Plus 2.4+ 内置的 el-table-v2 在 1 万行库存台账数据下的渲染性能与可用性,核心指标:首次渲染 < 500ms、滚动 FPS >= 50、内存 < 100MB。"
    />

    <VirtualStockTabPOC />
  </div>
</template>

<script setup lang="ts">
/**
 * 虚拟列表 POC 演示入口页
 */
import { useRouter } from 'vue-router'
import { ElButton } from 'element-plus'
import { Back } from '@element-plus/icons-vue'
import VirtualStockTabPOC from './tabs/VirtualStockTabPOC.vue'

const router = useRouter()
const goBack = () => {
  router.back()
}
</script>

<style scoped>
.inventory-poc-page {
  padding: 0;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: 16px;
  flex-wrap: wrap;
  gap: 12px;
}
.page-title {
  margin: 0 0 8px 0;
  font-size: 20px;
  font-weight: 600;
}
.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
.poc-alert {
  margin-bottom: 16px;
}
</style>
