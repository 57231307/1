<script setup lang="ts">
/**
 * PurchaseTop - 采购管理页顶部（标题 + 面包屑 + 操作按钮）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 页头）
 */
import { useI18n } from 'vue-i18n';
import { Plus, Printer, Download } from '@element-plus/icons-vue';

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' });

interface Props {
  onCreate: () => void;
  onPrint: () => void;
  onExport: () => void;
}

defineProps<Props>();
</script>

<template>
  <div class="page-header">
    <div class="header-left">
      <h1 class="page-title">{{ t('purchase.top.title') }}</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">{{
          t('purchase.top.breadcrumbHome')
        }}</el-breadcrumb-item>
        <el-breadcrumb-item>{{ t('purchase.top.breadcrumbPurchase') }}</el-breadcrumb-item>
        <el-breadcrumb-item>{{ t('purchase.top.breadcrumbOrder') }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="header-actions">
      <el-button type="primary" @click="onCreate">
        <el-icon><Plus /></el-icon>
        {{ t('purchase.top.create') }}
      </el-button>
      <el-button
        v-permission="'purchase.order.print'"
        @click="onPrint"
      >
        <el-icon><Printer /></el-icon>
        {{ t('purchase.top.print') }}
      </el-button>
      <el-button
        v-permission="'purchase.order.export'"
        @click="onExport"
      >
        <el-icon><Download /></el-icon>
        {{ t('purchase.top.export') }}
      </el-button>
    </div>
  </div>
</template>
