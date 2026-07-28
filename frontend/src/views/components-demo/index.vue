<template>
  <div class="component-demo-page">
    <el-tabs v-model="activeTab" type="border-card">
      <el-tab-pane :label="t('componentsDemo.tabs.charts')" name="charts">
        <ChartsDemo />
      </el-tab-pane>
      <el-tab-pane :label="t('componentsDemo.tabs.batch')" name="batch">
        <BatchActionsDemo />
      </el-tab-pane>
      <el-tab-pane :label="t('componentsDemo.tabs.filter')" name="filter">
        <AdvancedFilterDemo />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import ChartsDemo from './ChartsDemo.vue';
import BatchActionsDemo from './BatchActionsDemo.vue';
import AdvancedFilterDemo from './AdvancedFilterDemo.vue';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('charts');
</script>

<style scoped>
.component-demo-page {
  padding: 20px;
}
</style>
