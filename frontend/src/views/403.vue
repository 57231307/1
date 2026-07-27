<template>
  <div class="error-page" :aria-label="t('error.forbidden.pageAriaLabel')">
    <el-result
      icon="warning"
      :title="t('error.forbidden.title')"
      :sub-title="t('error.forbidden.message')"
    >
      <template #extra>
        <el-button type="primary" @click="$router.push('/')">{{
          t('error.forbidden.backHome')
        }}</el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });
</script>
