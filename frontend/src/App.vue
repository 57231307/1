<template>
  <ErrorBoundary :report="true">
    <router-view :aria-label="t('app.pageAriaLabel')" />
  </ErrorBoundary>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import ErrorBoundary from '@/components/ErrorBoundary.vue';

const { t } = useI18n({ useScope: 'global' });
</script>

<style>
body {
  margin: 0;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}
</style>
