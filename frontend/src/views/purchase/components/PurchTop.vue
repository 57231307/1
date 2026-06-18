<script setup lang="ts">
/**
 * PurchTop - 采购管理页顶部（标题 + 面包屑 + 操作按钮）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 页头）
 */
import { Plus, Printer, Download } from '@element-plus/icons-vue'

interface Props {
  onCreate: () => void
  onPrint: () => void
  onExport: () => void
}

defineProps<Props>()
</script>

<template>
  <div class="page-header">
    <div class="header-left">
      <h1 class="page-title">采购管理</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>采购管理</el-breadcrumb-item>
        <el-breadcrumb-item>采购订单</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="header-actions">
      <el-button type="primary" @click="onCreate">
        <el-icon><Plus /></el-icon>
        新建采购单
      </el-button>
      <el-button @click="onPrint">
        <el-icon><Printer /></el-icon>
        打印
      </el-button>
      <el-button @click="onExport">
        <el-icon><Download /></el-icon>
        导出
      </el-button>
    </div>
  </div>
</template>
