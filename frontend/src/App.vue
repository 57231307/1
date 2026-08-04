<template>
  <ErrorBoundary :report="true">
    <router-view :aria-label="t('app.pageAriaLabel')" />
  </ErrorBoundary>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import ErrorBoundary from '@/components/ErrorBoundary.vue';

const { t } = useI18n({ useScope: 'global' });

// V15 P2 20.12-C：浏览器空闲时预加载常用页面 chunk，减少路由切换延迟
onMounted(() => {
  const preload = () => {
    // 常用路由懒加载模块预取（失败静默忽略）
    import('./views/Dashboard.vue').catch(() => {});
    import('./views/custom-orders/list.vue').catch(() => {});
    import('./views/sales-contract/index.vue').catch(() => {});
  };
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(preload, { timeout: 3000 });
  } else {
    setTimeout(preload, 1000);
  }
});
</script>

<style>
body {
  margin: 0;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* V15 P3 Touch targets: 移动端按钮最小点击区域 ≥ 44px（WCAG 2.5.5） */
@media (pointer: coarse) {
  .el-button--small {
    min-height: 44px;
    min-width: 44px;
  }
}
</style>
